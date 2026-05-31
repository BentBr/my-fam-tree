import createClient, { type Middleware } from 'openapi-fetch'

import { i18n } from '@/i18n'
import { router } from '@/router'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import { ApiClientError, type ApiErrorBody, type Warning } from './errors'
import type { paths } from './schema'

// When the api hands back a 401 that can't be silently refreshed (refresh
// itself failed, or the code isn't `auth_token_expired` because the cookie
// was dropped server-side), clear the in-memory session and bounce to
// sign-in. This is the REDIRECT side-effect only — the user-facing
// "session expired" toast is owned by `reportError` in queryClient.ts so
// all error messaging lives in exactly one place. The route guard would
// catch them on the next navigation anyway; we redirect on the failed
// request itself so the stale authenticated UI doesn't linger.
function endSession(): void {
    const auth = useAuthStore()
    // If we're already anonymous, this is either initial hydrate or the
    // user is mid-sign-in. The router guards handle that flow on their
    // own and we must not race them by triggering a second navigation.
    if (auth.status === 'anonymous') return
    auth.applyClaimsPayload(null)
    // `replace` (not `push`) so the expired page never sits in history.
    // Don't await — middleware must finish before the caller awaits and
    // unmounts the source view. `void` satisfies `no-floating-promises`.
    void router.replace('/auth/sign-in')
}

// Parse an error response body into the typed `ApiClientError`. Shared by
// `errorTranslator` (the normal path) AND the `authRefresh` retry, so a
// request still failing AFTER a token refresh is translated identically
// instead of slipping through as a raw fetch response (the bug that made
// favourite-toggle 401s silent). Returns the error; callers `throw` it so
// TS sees an explicit throw at each site.
async function toApiError(response: Response): Promise<ApiClientError> {
    try {
        return new ApiClientError((await response.clone().json()) as ApiErrorBody)
    } catch {
        // Non-JSON error body — synthesise a minimal envelope so callers
        // still get an ApiClientError with the right status to branch on.
        return new ApiClientError({
            type: 'about:blank',
            title: response.statusText || 'Request failed',
            status: response.status,
            code: 'internal',
        } as ApiErrorBody)
    }
}

// Empty string ⇒ openapi-fetch issues same-origin requests. Both the host browser
// (`http://my-fam-tree.docker` → dinghy → fe:5173) and the in-network Playwright
// browser (`http://my-fam-tree.docker:5173`) hit the FE origin, and Vite's `/api`
// proxy forwards to the api service. Setting an absolute URL here would make the
// browser bypass the proxy and fail from inside the compose network where the
// api container only listens on 8080 (no port-80 routing inside docker).
const baseUrl = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? ''

const familyIdInjector: Middleware = {
    async onRequest({ request }) {
        const family = useActiveFamilyStore()
        const id = family.activeFamilyId
        if (id !== null && !request.headers.has('X-Family-Id')) {
            request.headers.set('X-Family-Id', id)
        }
        return request
    },
}

let refreshing: Promise<void> | null = null

const authRefresh: Middleware = {
    async onResponse({ request, response }) {
        if (response.status !== 401) return response
        // Avoid infinite recursion: /auth/refresh's own 401 must NOT
        // re-trigger this middleware (it would loop forever). Let it
        // bubble up so the caller (`auth.refresh()`) sees the 401 via
        // its own error path.
        if (new URL(request.url).pathname.endsWith('/auth/refresh')) {
            return response
        }
        let body: ApiErrorBody | undefined
        try {
            body = (await response.clone().json()) as ApiErrorBody
        } catch {
            return response
        }
        // Try refresh on ANY 401, not just `auth_token_expired`.
        //
        // The narrower "only on auth_token_expired" check fails the
        // common case where the access cookie has expired wall-clock-
        // wise: the BROWSER drops expired cookies, so the request
        // arrives at the BE with NO cookie at all — the BE answers
        // `auth_unauthenticated`, not `auth_token_expired`. The refresh
        // cookie has a separate, longer TTL and a narrower path
        // (`/api/v1/auth/refresh`), so it survives the access cookie's
        // expiry and is still available for the refresh round-trip.
        //
        // For genuinely-anonymous callers (no refresh cookie either),
        // the refresh attempt below itself returns 401 — we fall
        // through to `endSession()`, which is a no-op when the auth
        // store is already in the `anonymous` state, so the user just
        // sees a normal anonymous page instead of being bounced.
        //
        // The cost is one extra `/auth/refresh` round-trip on the very
        // first 401 of an anonymous session — fine for the
        // "session-resume-after-cold-tab" UX win it buys us.
        //
        // `body` is retained so we can rethrow it verbatim if the
        // refresh path fails — preserves the original error semantics
        // for `reportError`'s toast pipeline.
        const auth = useAuthStore()
        refreshing ??= auth
            .refresh()
            .catch((e) => {
                throw e
            })
            .finally(() => {
                refreshing = null
            })
        try {
            await refreshing
        } catch {
            // Refresh failed — the long-lived session is gone (or there
            // was none to begin with). Treat as hard logout: redirect
            // (no-op if already anonymous) + rethrow the ORIGINAL 401
            // body so `reportError` shows the right toast — e.g.,
            // "session expired" rather than "refresh invalid".
            endSession()
            throw new ApiClientError(body)
        }
        // Retry the original request now that the cookie is fresh. The
        // retried response does NOT re-enter this middleware chain
        // (onResponse already ran), so we translate failures here
        // ourselves rather than letting a still-failing retry escape
        // silently. A retry that's still 401 means the refresh didn't
        // actually recover the session → end it.
        const retried = await fetch(request)
        if (!retried.ok) {
            if (retried.status === 401) endSession()
            throw await toApiError(retried)
        }
        return retried
    },
}

const errorTranslator: Middleware = {
    async onResponse({ response }) {
        if (response.ok) return response
        const ct = response.headers.get('content-type') ?? ''
        if (!ct.includes('application/problem+json')) return response
        throw await toApiError(response)
    },
}

// Soft validations (e.g. sibling partnership, parent-child gap < 14y) ride
// along on the success envelope's `meta.warnings`. Surface them as info
// toasts so the user sees the heuristic flag without blocking the write.
//
// Only fires on mutating verbs — GETs returning the same payload on every
// refetch would otherwise spam the toast stack. Translation goes through
// the same i18n catalog as field violations, so adding a new warning code
// only needs a key in `en.json` / `de.json`.
const warningsBroadcaster: Middleware = {
    async onResponse({ request, response }) {
        if (request.method === 'GET') return response
        if (!response.ok) return response
        const ct = response.headers.get('content-type') ?? ''
        if (!ct.includes('application/json')) return response
        let envelope: { meta?: { warnings?: Warning[] } | null }
        try {
            envelope = (await response.clone().json()) as typeof envelope
        } catch {
            return response
        }
        const warnings = envelope.meta?.warnings ?? []
        if (warnings.length === 0) return response
        const ui = useUiStore()
        for (const w of warnings) {
            const msg = i18n.global.te(w.code) ? i18n.global.t(w.code) : w.message
            ui.pushToast({ kind: 'info', message: msg, code: w.code })
        }
        return response
    },
}

export const client = createClient<paths>({ baseUrl, credentials: 'include' })
client.use(familyIdInjector, authRefresh, errorTranslator, warningsBroadcaster)
