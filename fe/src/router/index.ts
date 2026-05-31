import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

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
        requiresAuth?: boolean
        requiresAdmin?: boolean
        /**
         * Marks a route as part of the unauthenticated public site.
         * Guards skip the sign-in bounce; the layout dispatcher picks
         * `PublicLayout`; signed-in users can still reach `/` (it's
         * informational, not a sign-in shortcut).
         */
        public?: boolean
    }
}

const routes: RouteRecordRaw[] = [
    {
        path: '/',
        name: 'home',
        component: () => import('@/views/public/HomeView.vue'),
        meta: { layout: 'public', public: true },
    },
    {
        path: '/imprint',
        name: 'imprint',
        component: () => import('@/views/public/ImprintView.vue'),
        meta: { layout: 'public', public: true },
    },
    {
        path: '/data-policy',
        name: 'data-policy',
        component: () => import('@/views/public/DataPolicyView.vue'),
        meta: { layout: 'public', public: true },
    },
    {
        path: '/auth/sign-in',
        name: 'sign-in',
        component: () => import('@/views/auth/LoginView.vue'),
        meta: { layout: 'login', requiresAuth: false },
    },
    {
        path: '/auth/consume',
        name: 'consume',
        component: () => import('@/views/auth/ConsumeView.vue'),
        meta: { layout: 'login', requiresAuth: false },
    },
    {
        path: '/families/create',
        name: 'family-create',
        component: () => import('@/views/families/FamilyCreate.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: '/families/pick',
        name: 'family-pick',
        component: () => import('@/views/families/FamilyPicker.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: '/invite/accept',
        name: 'invite-accept',
        component: () => import('@/views/auth/InviteAccept.vue'),
        meta: { layout: 'login', requiresAuth: false },
    },
    {
        path: '/health',
        name: 'health',
        component: () => import('@/views/HealthView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: false },
    },
    {
        path: '/account',
        name: 'account',
        component: () => import('@/views/account/AccountView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: '/account/email-change/consume',
        name: 'email-change-consume',
        component: () => import('@/views/account/EmailChangeConsumeView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: '/account/owner-transfer/confirm',
        name: 'owner-transfer-confirm',
        component: () => import('@/views/account/OwnerTransferConfirm.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: '/tree',
        name: 'tree',
        component: () => import('@/views/tree/TreeView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: '/upcoming',
        name: 'upcoming',
        component: () => import('@/views/upcoming/UpcomingView.vue'),
        meta: { layout: 'main', sidebar: 'main', requiresAuth: true },
    },
    {
        path: '/admin',
        redirect: '/admin/audit',
    },
    {
        path: '/admin/audit',
        name: 'admin-audit',
        component: () => import('@/views/admin/AdminAudit.vue'),
        meta: { layout: 'main', sidebar: 'admin', requiresAuth: true, requiresAdmin: true },
    },
    {
        path: '/admin/members',
        name: 'admin-members',
        component: () => import('@/views/admin/AdminMembers.vue'),
        meta: { layout: 'main', sidebar: 'admin', requiresAuth: true, requiresAdmin: true },
    },
    {
        path: '/admin/invites',
        name: 'admin-invites',
        component: () => import('@/views/admin/AdminInvites.vue'),
        meta: { layout: 'main', sidebar: 'admin', requiresAuth: true, requiresAdmin: true },
    },
    // /reminders/* etc. are added in Phase 4b.
]

export const router = createRouter({
    history: createWebHistory(),
    routes,
})

router.beforeEach(async (to) => {
    const auth = useAuthStore()
    if (auth.status === 'anonymous') {
        try {
            await auth.hydrate()
        } catch {
            // Network failure during hydrate — stay anonymous; the guard below
            // will bounce the request to sign-in.
        }
    }
    // `/invite/*` is also exempt: the InviteAccept view handles anonymous
    // arrivals itself by stashing the token to sessionStorage and bouncing the
    // user to /auth/sign-in. If the gate bounced first, the token would be
    // dropped from the URL before InviteAccept ever saw it.
    // Routes flagged `meta.public` (the marketing + legal pages) are also
    // exempt: they're informational and signed-in users may still browse
    // them. Anonymous visitors stay on them too.
    const isExempt = to.meta.public === true || to.path.startsWith('/auth/') || to.path.startsWith('/invite/')
    if (auth.status === 'anonymous' && !isExempt) {
        return '/auth/sign-in'
    }
    if (auth.status === 'authenticated' && to.path === '/auth/sign-in') {
        // Don't keep showing the sign-in page once we're logged in.
        return '/health'
    }
    return true
})

router.beforeEach((to) => {
    const auth = useAuthStore()
    const family = useActiveFamilyStore()
    if (auth.status !== 'authenticated') return true
    // Reconcile stale active-family BEFORE the exempt check: localStorage
    // may carry an `activeFamilyId` from a previous session whose
    // membership no longer exists in `auth.families` (the user got
    // removed, the family was deleted, or the run signed in as a
    // different identity on the same browser). Letting the tree query
    // fire with that stale id triggers a 422 X-Family-Id validation on
    // the API, surfacing as the toast "Validation failed" — and the
    // switcher shows the raw UUID because no item title matches. Wipe
    // it here so even exempt routes (e.g. `/account`, `/health`) don't
    // leak the stale id into their FamilySwitcher render.
    if (family.activeFamilyId !== null && !auth.families.some((f) => f.id === family.activeFamilyId)) {
        family.clearOnLogout()
    }
    // Routes that don't need an active family:
    //   - `meta.public` marketing / legal pages
    //   - `/auth/*` — sign-in / consume / refresh flows
    //   - `/families/*` — picker / create themselves resolve the family
    //   - `/invite/*` — token redemption may happen before the user has any
    //     family at all
    //   - `/account` — user-scoped profile / locale / avatar; no family
    //     context needed and a fresh user (zero families) must be able to
    //     reach it directly without being bounced through /families/create
    //   - `/health` — status page, no family context
    //
    // Auto-select-when-sole-family runs BEFORE the exempt check too: even
    // routes that don't *require* an active family still want one set so
    // the AppBar's FamilySwitcher reflects the user's family on /account
    // / /health renders.
    if (family.activeFamilyId === null && auth.families.length === 1) {
        const sole = auth.families[0]
        if (sole !== undefined) {
            family.setActive(sole.id)
        }
    }
    const isExempt =
        to.meta.public === true ||
        to.path.startsWith('/auth/') ||
        to.path.startsWith('/families/') ||
        to.path.startsWith('/invite/') ||
        to.path.startsWith('/account') ||
        to.path === '/health'
    if (isExempt) return true
    if (family.activeFamilyId !== null) return true
    // Non-exempt routes need an active family. A zero-family user falls
    // through to /families/create; the multi-family case sends them to
    // the picker. The sole-family auto-select above already handled the
    // common single-family path.
    return auth.families.length === 0 ? '/families/create' : '/families/pick'
})

// Admin-only role gate. Runs after the auth + active-family guards so by
// the time we reach it we know there's a session and an active family.
// Non-admin / non-owner roles are bounced to /tree; this matches the
// pattern Vue Router expects for a "redirect when condition fails"
// guard (return a path).
router.beforeEach((to) => {
    if (to.meta.requiresAdmin !== true) return true
    const family = useActiveFamilyStore()
    const role = family.activeFamily?.role ?? null
    if (role === 'admin' || role === 'owner') return true
    return '/tree'
})
