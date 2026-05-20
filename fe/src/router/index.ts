import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'

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
        // @ts-expect-error -- view lands in Task 8
        component: () => import('@/views/auth/LoginView.vue'),
        meta: { layout: 'login', requiresAuth: false },
    },
    {
        path: '/auth/consume',
        name: 'consume',
        // @ts-expect-error -- view lands in Task 8
        component: () => import('@/views/auth/ConsumeView.vue'),
        meta: { layout: 'login', requiresAuth: false },
    },
    {
        path: '/health',
        name: 'health',
        // @ts-expect-error -- view lands in Task 8
        component: () => import('@/views/HealthView.vue'),
        meta: { layout: 'main', requiresAuth: false },
    },
    // /tree, /reminders, etc. are added in Phase 1b / 2b / 4b.
]

export const router = createRouter({
    history: createWebHistory(),
    routes,
})

// Auth + family guards land in Phase 1b. Phase 0d's stub view set
// (sign-in, consume, health) is reachable without authentication.
