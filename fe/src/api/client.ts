import createClient, { type Middleware } from 'openapi-fetch'

import { i18n } from '@/i18n'
import { router } from '@/router'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import { ApiClientError, type ApiErrorBody, type Warning } from './errors'
import type { paths } from './schema'

// On any 401 the FE cannot silently refresh (refresh itself failed
// too), clear the in-memory session, drop the HttpOnly cookies
// server-side, and bounce to sign-in.
//
// HttpOnly cookies cannot be cleared from JavaScript — only the BE's
// `Set-Cookie max-age=0` response drops them from the browser. We
// POST `/auth/logout` (public + idempotent — see
// `crates/api/src/routes/mod.rs`) which always emits the clearing
// headers; without this, stale refresh cookies would sit in the
// browser until their 30-day TTL and a returning user would keep
// hitting the same failure loop.
//
// The user-facing "session expired" toast is owned by `reportError`
// in queryClient.ts so all error messaging lives in exactly one
// place. The route guard would catch them on the next navigation
// anyway; we redirect on the failed request itself so the stale
// authenticated UI doesn't linger.
function endSession(): void {
    const auth = useAuthStore()
    // Already-anonymous callers (initial hydrate, mid-sign-in flows)
    // skip everything: there's no session for the route guards to
    // race against, and a never-signed-in caller has no cookies for
    // the BE wipe to clear (any cross-tab staleness rides out the
    // cookie's natural TTL).
    if (auth.status === 'anonymous') return
    // Sync local-state flip so route guards on the next tick see
    // `anonymous` immediately and don't race with the async BE call.
    auth.applyClaimsPayload(null)
    // Async BE cookie wipe via the public /auth/logout endpoint. The
    // store's `logout()` wraps the BE call with the localStorage /
    // sessionStorage cleanup so app-owned state goes too. Fire-and-
    // forget: a network failure must not block the redirect, and the
    // in-flight request's response doesn't need to wait on this
    // side-effect. The redundant `applyClaimsPayload(null)` inside
    // `auth.logout()` is a harmless idempotent safety net.
    void auth.logout()
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

// Each in-flight Request gets a `.clone()` stashed in this WeakMap on
// the `onRequest` hook. After a 401 → refresh round-trip we use the
// CLONE to retry — the original Request's body stream has already
// been consumed by the first fetch and reusing it would either
// silently drop the body (Chrome) or throw "Body is unusable"
// (Firefox / strict spec), so retried POST/PATCH/DELETE calls would
// land at the BE with an empty payload and fail validation in a way
// that looks identical to a still-expired session.
//
// WeakMap keyed by the Request object means the clone is GC'd as soon
// as the original drops out of scope — no leak across the long-lived
// client.
const retryClones = new WeakMap<Request, Request>()

const requestCloner: Middleware = {
    async onRequest({ request }) {
        // `clone()` snapshots method + url + headers + body for a
        // single replay. Future replays (rare, but possible if
        // refresh races) would need another clone — out of scope
        // here. GET / HEAD have no body and the clone is cheap.
        retryClones.set(request, request.clone())
        return request
    },
}

const authRefresh: Middleware = {
    async onResponse({ request, response }) {
        if (response.status !== 401) return response
        // Skip refresh for endpoints where 401 has nothing to do with
        // session expiry:
        //   - /auth/refresh — recursive: refreshing the refresh would
        //     loop forever. Let the 401 bubble up to `auth.refresh()`'s
        //     own error path.
        //   - /invites/accept — the invite TOKEN is the auth factor
        //     here, not the session cookie. 401 means "this invite is
        //     invalid / already consumed", not "your session expired";
        //     retrying with a fresh access cookie can't help (the BE
        //     still rejects the same single-use token) and would just
        //     consume the refresh-rate budget plus, worse, end the
        //     session for an already-signed-in user who clicked a
        //     stale invite link by mistake.
        const path = new URL(request.url).pathname
        if (path.endsWith('/auth/refresh') || path.endsWith('/invites/accept')) {
            return response
        }
        let body: ApiErrorBody | undefined
        try {
            body = (await response.clone().json()) as ApiErrorBody
        } catch {
            return response
        }
        // Try refresh on ANY 401 (except `/auth/refresh` itself, handled
        // above). The access cookie's TTL is short and the BROWSER drops
        // expired cookies from outgoing requests, so a 401 here can mean
        // "no cookie at all" (BE returns `auth_unauthenticated`) just
        // as easily as "JWT expired" (`auth_token_expired`). The refresh
        // cookie has a longer TTL and a narrower path
        // (`/api/v1/auth/refresh`), so it survives the access cookie's
        // expiry and is still available for the refresh round-trip.
        //
        // Genuinely-anonymous callers (no refresh cookie either) get a
        // 401 on the refresh attempt itself and fall through to
        // `endSession()` — which short-circuits when the auth store is
        // already `anonymous`, so a never-signed-in visitor on a
        // public page stays put instead of being bounced.
        //
        // Cost: one extra `/auth/refresh` round-trip on the first 401
        // of a cold session — small price for the session-resume-
        // after-cold-tab UX win.
        //
        // `body` is retained so the refresh-fail path can rethrow it
        // verbatim, preserving the original error semantics for
        // `reportError`'s toast pipeline.
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
        //
        // CRITICAL: use the cloned request stashed by `requestCloner`,
        // NOT the original. The original's body stream was consumed by
        // the first fetch and replaying it would land an empty body at
        // the BE for any POST/PATCH/DELETE that needed to refresh
        // mid-flight (creating a person, uploading an avatar, …) —
        // exactly the case where preserving the user's pending write
        // matters most. Fall back to the original Request if the clone
        // is somehow missing (a future middleware re-write that strips
        // requestCloner would otherwise silently regress to broken
        // retries for GETs).
        const replay = retryClones.get(request) ?? request
        const retried = await fetch(replay)
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

// Middleware order matters in two directions:
//
//   onRequest:  runs in the order passed to `use()`.
//   onResponse: runs in REVERSE order.
//
// We need `authRefresh` to be the FIRST middleware that sees a 401
// response — otherwise `errorTranslator` would throw the typed error
// before `authRefresh` could intercept and retry, and a token-expiry
// 401 would bounce the user to /auth/sign-in instead of refreshing
// silently. Putting `authRefresh` LAST in the `use()` call places it
// FIRST in the onResponse chain.
//
// `requestCloner` runs AFTER `familyIdInjector` in the onRequest
// chain so the clone captures the X-Family-Id header (and any other
// header any future request-side middleware adds). Otherwise the
// replayed request would land at the BE without the active-family
// selector and 422 against the X-Family-Id validator. The cloner has
// no onResponse — it's a pure request-side helper that just stashes
// a replay copy in the per-request WeakMap.
//
//   onRequest:  familyIdInjector → requestCloner → errorTranslator → warningsBroadcaster → authRefresh
//   onResponse: authRefresh → warningsBroadcaster → errorTranslator → requestCloner → familyIdInjector
client.use(familyIdInjector, requestCloner, errorTranslator, warningsBroadcaster, authRefresh)
