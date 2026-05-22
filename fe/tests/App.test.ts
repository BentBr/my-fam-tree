import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import App from '@/App.vue'
import { i18n } from '@/i18n'

async function mountWithLayout(layout: 'login' | 'main' | undefined): Promise<ReturnType<typeof mount>> {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            {
                path: '/',
                component: { template: '<div />' },
                meta: layout === undefined ? {} : { layout },
            },
        ],
    })
    await router.push('/')
    await router.isReady()
    return mount(App, {
        global: {
            plugins: [createPinia(), i18n, router],
            stubs: {
                'v-app': { template: '<div class="vapp"><slot /></div>' },
                LoginLayout: { template: '<div class="login-stub" />' },
                MainLayout: { template: '<div class="main-stub" />' },
                ToastContainer: { template: '<div class="toast-stub" />' },
            },
        },
    })
}

describe('App.vue layout switch', () => {
    it('renders LoginLayout when route.meta.layout === "login"', async () => {
        const w = await mountWithLayout('login')
        expect(w.find('.login-stub').exists()).toBe(true)
        expect(w.find('.main-stub').exists()).toBe(false)
    })

    it('renders MainLayout when meta is "main"', async () => {
        const w = await mountWithLayout('main')
        expect(w.find('.main-stub').exists()).toBe(true)
    })

    it('defaults to MainLayout when meta is missing', async () => {
        const w = await mountWithLayout(undefined)
        expect(w.find('.main-stub').exists()).toBe(true)
    })

    it('always mounts ToastContainer outside the layout switch', async () => {
        const w = await mountWithLayout('login')
        expect(w.find('.toast-stub').exists()).toBe(true)
    })
})
