import createClient, { type Middleware } from 'openapi-fetch'

import { i18n } from '@/i18n'
import { router } from '@/router'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import { ApiClientError, type ApiErrorBody, type Warning } from './errors'
import type { paths } from './schema'

// When the api hands back a 401 that can't be silently refreshed (refresh
// itself failed, or the code is `auth_required` because the cookie was
// dropped server-side), clear the in-memory session and bounce to sign-in
// immediately. Without this the user sees a stale authenticated UI plus
// an inline "authentication required" toast and has to manually sign out.
// The route guard would catch them on the next navigation; we want the
// redirect to happen on the failed click itself.
function endSessionAndRedirect(): void {
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

// Empty string ⇒ openapi-fetch issues same-origin requests. Both the host browser
// (`http://my-family.docker` → dinghy → fe:5173) and the in-network Playwright
// browser (`http://my-family.docker:5173`) hit the FE origin, and Vite's `/api`
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
        let body: ApiErrorBody | undefined
        try {
            body = (await response.clone().json()) as ApiErrorBody
        } catch {
            return response
        }
        if (body.code !== 'auth_token_expired') {
            endSessionAndRedirect()
            throw new ApiClientError(body)
        }
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
        } catch (e) {
            // Refresh failed — the long-lived session is gone. Treat as
            // hard logout: kicking the user to sign-in is the only safe
            // next step.
            endSessionAndRedirect()
            throw e
        }
        return fetch(request)
    },
}

const errorTranslator: Middleware = {
    async onResponse({ response }) {
        if (response.ok) return response
        const ct = response.headers.get('content-type') ?? ''
        if (!ct.includes('application/problem+json')) return response
        const body = (await response.clone().json()) as ApiErrorBody
        throw new ApiClientError(body)
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
