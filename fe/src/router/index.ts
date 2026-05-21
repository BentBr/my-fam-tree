import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

declare module 'vue-router' {
    interface RouteMeta {
        layout?: 'login' | 'main'
        requiresAuth?: boolean
    }
}

const routes: RouteRecordRaw[] = [
    { path: '/', redirect: '/tree' },
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
        meta: { layout: 'main', requiresAuth: true },
    },
    {
        path: '/families/pick',
        name: 'family-pick',
        component: () => import('@/views/families/FamilyPicker.vue'),
        meta: { layout: 'main', requiresAuth: true },
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
        meta: { layout: 'main', requiresAuth: false },
    },
    {
        path: '/account',
        name: 'account',
        component: () => import('@/views/account/AccountView.vue'),
        meta: { layout: 'main', requiresAuth: true },
    },
    {
        path: '/account/email-change/consume',
        name: 'email-change-consume',
        component: () => import('@/views/account/EmailChangeConsumeView.vue'),
        meta: { layout: 'main', requiresAuth: true },
    },
    {
        path: '/tree',
        name: 'tree',
        component: () => import('@/views/tree/TreeView.vue'),
        meta: { layout: 'main', requiresAuth: true },
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
    const isExempt = to.path.startsWith('/auth/') || to.path.startsWith('/invite/')
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
    const isExempt = to.path.startsWith('/auth/') || to.path.startsWith('/families/') || to.path.startsWith('/invite/')
    if (isExempt) return true
    if (family.activeFamilyId !== null) return true
    return auth.families.length === 0 ? '/families/create' : '/families/pick'
})
