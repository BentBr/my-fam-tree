import type { LocationQuery, RouteLocationRaw, RouteRecordRaw, Router } from 'vue-router'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { detectInitialLocale, useLocaleStore, type SupportedLocale } from '@/stores/locale'

declare module 'vue-router' {
    interface RouteMeta {
        layout?: 'login' | 'main' | 'public'
        /**
         * Drives `AppSidebar`'s visible variant. `'main'` shows the
         * tree-side nav, `'admin'` shows the admin items, `'none'`
         * hides the drawer entirely. Defaults to `'none'` so any
         * unmarked route gets the chromeless behaviour by accident
         * rather than a half-rendered sidebar.
         */
        sidebar?: 'main' | 'admin' | 'none'
        /**
         * Session requirement. The auth guard lets anonymous visitors
         * stay only on routes that are `public` OR explicitly
         * `requiresAuth: false` (the auth-flow pages that handle the
         * anonymous case themselves). Every other route — including any
         * route that omits this flag — needs a session.
         */
        requiresAuth?: boolean
        /**
         * Active-family requirement. Only routes flagged
         * `requiresFamily` bounce a family-less (but authenticated) user
         * to `/families/create` or `/families/pick`. Family-optional
         * authed pages (account, health, the family picker itself) omit
         * it. `requiresAdmin` implies `requiresFamily` (a role lives in a
         * family), so admin routes set both.
         */
        requiresFamily?: boolean
        requiresAdmin?: boolean
        /**
         * Marks a route as part of the unauthenticated public site.
         * Guards skip the sign-in bounce; the layout dispatcher picks
         * `PublicLayout`; signed-in users can still reach `/en` (it's
         * informational, not a sign-in shortcut). The public routes are
         * the ones vite-ssg prerenders with correct-locale metadata
         * (see `main.ts` + `vite.config.ts`).
         */
        public?: boolean
        /**
         * Content-width tier for the `MainLayout` page container. Drives one
         * coherent system instead of per-view max-widths:
         *   - `'full'`    — no clamp, fills the canvas (the tree).
         *   - `'wide'`    — clamped wide + centered (data tables: audit,
         *     members, invites, upcoming) so they fit the screen but don't
         *     sprawl on ultrawide.
         *   - `'reading'` — clamped narrow + centered (forms / overview pages:
         *     account, family settings, health, …). The DEFAULT when omitted.
         * `public`/`login` layouts ignore this (they own their own width).
         */
        width?: 'full' | 'wide' | 'reading'
    }
}

/**
 * EVERY route is locale-prefixed (`/en/tree`, `/de/imprint`, …) — the URL
 * is the single source of truth for the display locale. A catch-all
 * normalizer (last route record) redirects anything else:
 *
 *   - bare paths (`/`, `/tree`, the backend-minted magic-link /
 *     invite / email-change URLs) → the resolved-locale variant, query +
 *     hash preserved;
 *   - unknown locale segments (`/fr/imprint`) → the visitor's default
 *     locale with the same suffix.
 *
 * "Resolved locale" = stored preference (localStorage) → browser
 * language → English (see `detectInitialLocale`). Because programmatic
 * `router.push('/tree')` calls resolve through the same normalizer, view
 * code keeps writing clean bare paths and never carries the prefix.
 * Only the PUBLIC pages are prerendered; the rest stays a client SPA.
 */
const LANG = ':lang(en|de)'

// A path's first segment that *looks like* a locale tag (`fr`, `pt-BR`)
// but isn't one we support. Real app segments (`auth`, `tree`, `admin`,
// …) are all longer than two letters, so this can't shadow a page.
const LOCALE_LIKE = /^[a-z]{2}(-[a-z]{2})?$/i

/**
 * Normalize a non-matching path onto the locale-prefixed route space.
 * Implements the two redirect rules documented above; a known-locale
 * prefix with an unknown remainder falls back to that locale's home.
 */
function normalizeToLocale(to: { path: string; query: LocationQuery; hash: string }): RouteLocationRaw {
    const segments = to.path.split('/').filter((s) => s !== '')
    const first = segments[0] ?? ''
    if (first === 'en' || first === 'de') {
        // Supported locale but no route matched the remainder → home.
        return { path: `/${first}` }
    }
    const rest = LOCALE_LIKE.test(first) ? segments.slice(1) : segments
    const suffix = rest.length > 0 ? `/${rest.join('/')}` : ''
    return { path: `/${preferredLocale()}${suffix}`, query: to.query, hash: to.hash }
}

/**
 * Locale for a path that carries none (bare `/`, …). For a signed-in user
 * their ACCOUNT locale wins (it's authoritative — the enforce-guard would
 * redirect to it anyway, so resolving to it here avoids a double hop);
 * otherwise the anonymous chain (`detectInitialLocale`: localStorage →
 * browser → en). Auth is skipped under SSR (no session during prerender).
 */
function preferredLocale(): SupportedLocale {
    if (!import.meta.env.SSR) {
        const account = useAuthStore().user?.locale
        if (account === 'en' || account === 'de') return account
    }
    return detectInitialLocale()
}

const routes: RouteRecordRaw[] = [
    // ---- Public (prerendered per locale) ----
    {
        path: `/${LANG}`,
        name: 'home',
        component: () => import('@/views/public/HomeView.vue'),
        meta: { layout: 'public', public: true },
    },
    {
        path: `/${LANG}/imprint`,
        name: 'imprint',
        component: () => import('@/views/public/ImprintView.vue'),
        meta: { layout: 'public', public: true },
    },
    {
        path: `/${LANG}/data-policy`,
        name: 'data-policy',
        component: () => import('@/views/public/DataPolicyView.vue'),
        meta: { layout: 'public', public: true },
    },
    // ---- Authenticated app (client-rendered, still locale-prefixed) ----
    {
        path: `/${LANG}/auth/sign-in`,
        name: 'sign-in',
        component: () => import('@/views/auth/LoginView.vue'),
        meta: { layout: 'login', requiresAuth: false },
    },
    {
        path: `/${LANG}/auth/consume`,
        name: 'consume',
        component: () => import('@/views/auth/ConsumeView.vue'),
        meta: { layout: 'login', requiresAuth: false },
    },
    {
        path: `/${LANG}/families/create`,
        name: 'family-create',
        component: () => import('@/views/families/FamilyCreate.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: `/${LANG}/families/pick`,
        name: 'family-pick',
        component: () => import('@/views/families/FamilyPicker.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: `/${LANG}/invite/accept`,
        name: 'invite-accept',
        component: () => import('@/views/auth/InviteAccept.vue'),
        meta: { layout: 'login', requiresAuth: false },
    },
    {
        path: `/${LANG}/health`,
        name: 'health',
        component: () => import('@/views/HealthView.vue'),
        // Authenticated status page (renders in the main chrome and is the
        // post-login landing). Family-optional: no `requiresFamily`.
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: `/${LANG}/account`,
        name: 'account',
        component: () => import('@/views/account/AccountView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: `/${LANG}/account/email-change/consume`,
        name: 'email-change-consume',
        component: () => import('@/views/account/EmailChangeConsumeView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: `/${LANG}/account/owner-transfer/confirm`,
        name: 'owner-transfer-confirm',
        component: () => import('@/views/account/OwnerTransferConfirm.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: `/${LANG}/tree`,
        name: 'tree',
        component: () => import('@/views/tree/TreeView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true, requiresFamily: true, width: 'full' },
    },
    {
        path: `/${LANG}/upcoming`,
        name: 'upcoming',
        component: () => import('@/views/upcoming/UpcomingView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true, requiresFamily: true, width: 'wide' },
    },
    {
        path: `/${LANG}/admin`,
        redirect: (to): string => `/${String(to.params['lang'])}/admin/family`,
    },
    {
        path: `/${LANG}/admin/family`,
        name: 'admin-family',
        component: () => import('@/views/admin/AdminFamily.vue'),
        meta: { layout: 'main', sidebar: 'admin', requiresAuth: true, requiresFamily: true, requiresAdmin: true },
    },
    {
        path: `/${LANG}/admin/audit`,
        name: 'admin-audit',
        component: () => import('@/views/admin/AdminAudit.vue'),
        meta: {
            layout: 'main',
            sidebar: 'admin',
            requiresAuth: true,
            requiresFamily: true,
            requiresAdmin: true,
            width: 'wide',
        },
    },
    {
        path: `/${LANG}/admin/members`,
        name: 'admin-members',
        component: () => import('@/views/admin/AdminMembers.vue'),
        meta: {
            layout: 'main',
            sidebar: 'admin',
            requiresAuth: true,
            requiresFamily: true,
            requiresAdmin: true,
            width: 'wide',
        },
    },
    {
        path: `/${LANG}/admin/invites`,
        name: 'admin-invites',
        component: () => import('@/views/admin/AdminInvites.vue'),
        meta: {
            layout: 'main',
            sidebar: 'admin',
            requiresAuth: true,
            requiresFamily: true,
            requiresAdmin: true,
            width: 'wide',
        },
    },
    // /reminders/* etc. are added in Phase 4b.
    // ---- Locale normalizer (must stay LAST — lowest match priority) ----
    // Catches every path the records above don't: bare app paths,
    // backend-minted link URLs, unknown locale prefixes, typos. See
    // `normalizeToLocale` for the redirect rules.
    {
        path: '/:pathMatch(.*)*',
        name: 'locale-normalizer',
        redirect: normalizeToLocale,
    },
]

export { routes }

/**
 * Register the navigation guards on the router instance. Under
 * `vite-ssg` the router is created by the framework from `routes`, so we
 * can no longer `createRouter()` here and attach guards inline — the boot
 * (`main.ts`) calls this with the instance vite-ssg hands it.
 *
 * The session-dependent guards are a CLIENT concern: they hydrate the
 * session (a network call), reconcile the active family, and gate admin
 * routes. None of that is meaningful during a static prerender (no
 * backend, no cookies), and firing `auth.hydrate()` there would hang the
 * build — so those guards short-circuit under SSR. The locale-seeding
 * guard runs in BOTH environments so the prerendered public page is
 * localised from its URL prefix.
 */
export function registerGuards(router: Router): void {
    // Every route carries the locale in the URL (`/en/tree`, `/de/imprint`).
    // Seed the DISPLAY locale from that param on BOTH server (prerender) and
    // client so the rendered head + copy match the URL. `applyFromUrl` does
    // NOT persist — the URL is the highest-priority source for the page being
    // shown, but merely viewing or following a `/en` link must not overwrite
    // the user's stored `de` preference (the bug where opening `/` then
    // bouncing through `/en` reset the saved choice). Only explicit choices
    // (language menu, account form, account locale on sign-in) call `set()`,
    // which persists.
    router.beforeEach((to) => {
        const lang = typeof to.params['lang'] === 'string' ? to.params['lang'] : undefined
        if (lang === 'en' || lang === 'de') {
            useLocaleStore().applyFromUrl(lang)
        }
        return true
    })

    // Guard-internal redirect target in the locale of the navigation
    // being processed (the seeding guard above has already synced the
    // store to `to.params.lang`). Explicit prefix instead of bouncing
    // through the catch-all normalizer — one redirect hop, not two.
    const localized = (path: string): string => `/${useLocaleStore().locale}${path}`

    // Session hydrate + anonymous bounce — client only.
    router.beforeEach(async (to) => {
        if (import.meta.env.SSR) return true
        const auth = useAuthStore()
        // Captured BEFORE hydrate so the post-hydrate comparison sees the
        // un-narrowed union (TS keeps the `'anonymous'` narrowing across
        // the await and would reject the equality otherwise).
        const wasAnonymous = auth.status === 'anonymous'
        if (wasAnonymous) {
            try {
                await auth.hydrate()
            } catch (e) {
                // `hydrate()` already maps a 401 to "anonymous" and returns; if
                // we land here it's an UNEXPECTED failure (network down, 5xx).
                // Don't swallow it silently — log for diagnostics. We still
                // fall through to the anonymous bounce below: rethrowing from a
                // guard would abort navigation and strand the user with no
                // route, which is worse than a sign-in bounce.
                console.error('[router] unexpected auth hydrate failure', e)
            }
        }
        // Account locale is AUTHORITATIVE for a signed-in user: their stored
        // preference is enforced on every navigation, so the URL prefix can't
        // drift away from it (the discrepancy where account said `en` but the
        // URL / display / localStorage were all `de`). Changing the language
        // (menu or account form) updates the account first, so this never
        // fights an in-flight switch. The URL-highest rule still governs the
        // ANONYMOUS public site — this branch only runs once authenticated.
        if (auth.status === 'authenticated') {
            const pref = auth.user?.locale
            const urlLang = to.params['lang']
            if ((pref === 'en' || pref === 'de') && typeof urlLang === 'string' && urlLang !== pref) {
                // `fullPath` always starts with `/en` or `/de` here (3 chars) —
                // slicing keeps the suffix INCLUDING query + hash intact.
                return `/${pref}${to.fullPath.slice(3)}`
            }
        }
        // Anonymous visitors may remain only on:
        //   - `meta.public` routes — marketing + legal pages, also browsable
        //     while signed in;
        //   - `meta.requiresAuth === false` routes — the auth-flow pages
        //     (sign-in / consume / invite-accept) that handle the anonymous
        //     case themselves (e.g. InviteAccept stashes its token and bounces
        //     to sign-in; bouncing here first would drop the token from the URL).
        // Any other route — including one that omits the flag — needs a session.
        const anonymousAllowed = to.meta.public === true || to.meta.requiresAuth === false
        if (auth.status === 'anonymous' && !anonymousAllowed) {
            return localized('/auth/sign-in')
        }
        if (auth.status === 'authenticated' && to.name === 'sign-in') {
            // Don't keep showing the sign-in page once we're logged in.
            return localized('/health')
        }
        return true
    })

    // Active-family reconcile + requires-family gate — client only.
    router.beforeEach((to) => {
        if (import.meta.env.SSR) return true
        const auth = useAuthStore()
        const family = useActiveFamilyStore()
        if (auth.status !== 'authenticated') return true
        // Reconcile stale active-family BEFORE the requires-family check:
        // localStorage may carry an `activeFamilyId` from a previous session
        // whose membership no longer exists in `auth.families` (the user got
        // removed, the family was deleted, or the run signed in as a
        // different identity on the same browser). Letting the tree query
        // fire with that stale id triggers a 422 X-Family-Id validation on
        // the API, surfacing as the toast "Validation failed" — and the
        // switcher shows the raw UUID because no item title matches. Wipe
        // it here so even family-optional routes (e.g. `/account`, `/health`)
        // don't leak the stale id into their FamilySwitcher render.
        if (family.activeFamilyId !== null && !auth.families.some((f) => f.id === family.activeFamilyId)) {
            family.clearOnLogout()
        }
        // Auto-select-when-sole-family runs for every authed route (not just
        // the family-required ones) so the AppBar's FamilySwitcher reflects
        // the user's family on /account / /health renders too.
        if (family.activeFamilyId === null && auth.families.length === 1) {
            const sole = auth.families[0]
            if (sole !== undefined) {
                family.setActive(sole.id)
            }
        }
        // Only routes that declare `meta.requiresFamily` enforce an active
        // family. Family-optional authed pages (account*, health, the family
        // picker/create themselves, auth + invite flows, public pages) omit
        // the flag and pass through — driven by the route declaration rather
        // than a duplicated path-prefix allowlist.
        if (to.meta.requiresFamily !== true) return true
        if (family.activeFamilyId !== null) return true
        // Non-exempt routes need an active family. A zero-family user falls
        // through to /families/create; the multi-family case sends them to
        // the picker. The sole-family auto-select above already handled the
        // common single-family path.
        return auth.families.length === 0 ? localized('/families/create') : localized('/families/pick')
    })

    // Admin-only role gate. Runs after the auth + active-family guards so by
    // the time we reach it we know there's a session and an active family.
    // Non-admin / non-owner roles are bounced to /tree; this matches the
    // pattern Vue Router expects for a "redirect when condition fails"
    // guard (return a path).
    router.beforeEach((to) => {
        if (import.meta.env.SSR) return true
        if (to.meta.requiresAdmin !== true) return true
        const family = useActiveFamilyStore()
        const role = family.activeFamily?.role ?? null
        if (role === 'admin' || role === 'owner') return true
        return localized('/tree')
    })
}
