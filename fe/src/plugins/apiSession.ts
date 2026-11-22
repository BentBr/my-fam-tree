import { configureClient } from '@/api/client'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

/**
 * Connects the API client to the session stores. The stores import the
 * client, so the client cannot import them back (import cycle); this
 * composition-root module injects them instead. Stores resolve through the
 * active Pinia at call time, exactly as the client did before.
 *
 * `navigate` performs the sign-in redirect: the router is created per render
 * under vite-ssg (no singleton), so `main.ts` passes `router.replace` in the
 * browser and a no-op during SSR/prerender, where a redirect is meaningless.
 */
export function installApiSession(navigate: (to: string) => void): void {
    configureClient({
        familyId: () => useActiveFamilyStore().activeFamilyId,
        refreshSession: () => useAuthStore().refresh(),
        endSession: () => {
            endSession(navigate)
        },
        notifyWarning: (warning, message) => {
            useUiStore().pushToast({ kind: 'info', message, code: warning.code })
        },
    })
}

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
function endSession(navigate: (to: string) => void): void {
    const auth = useAuthStore()
    // Already-anonymous callers (initial hydrate, mid-sign-in flows)
    // skip everything: there's no session for the route guards to
    // race against, and a never-signed-in caller has no cookies for
    // the BE wipe to clear.
    if (auth.status === 'anonymous') return
    // Sync local-state flip so route guards on the next tick see
    // `anonymous` immediately and don't race with the async BE call.
    auth.applyClaimsPayload(null)
    // Fire-and-forget BE cookie wipe + app-owned storage cleanup: a
    // network failure must not block the redirect.
    void auth.logout()
    navigate('/auth/sign-in')
}
