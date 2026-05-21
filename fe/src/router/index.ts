import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'

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
        path: '/health',
        name: 'health',
        component: () => import('@/views/HealthView.vue'),
        meta: { layout: 'main', requiresAuth: false },
    },
    // /tree, /reminders, etc. are added in Phase 1b / 2b / 4b.
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
    const isAuthRoute = to.path.startsWith('/auth/')
    if (auth.status === 'anonymous' && !isAuthRoute) {
        return '/auth/sign-in'
    }
    if (auth.status === 'authenticated' && to.path === '/auth/sign-in') {
        // Don't keep showing the sign-in page once we're logged in.
        return '/health'
    }
    return true
})
