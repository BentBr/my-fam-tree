import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import MainLayout from '@/layouts/MainLayout.vue'

describe('MainLayout', () => {
    it('mounts AppBar + AppSidebar + router-view container', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: { template: '<div>content</div>' } }],
        })
        await router.push('/')
        await router.isReady()

        const wrapper = mount(MainLayout, {
            global: {
                plugins: [createPinia(), i18n, router],
                stubs: {
                    AppBar: { template: '<div class="appbar-stub" />' },
                    AppSidebar: { template: '<div class="sidebar-stub" />' },
                    'v-main': { template: '<main><slot /></main>' },
                    'v-container': { template: '<div><slot /></div>' },
                    'router-view': { template: '<div class="rv-stub" />' },
                },
            },
        })
        expect(wrapper.find('.appbar-stub').exists()).toBe(true)
        expect(wrapper.find('.sidebar-stub').exists()).toBe(true)
    })

    it('renders the routed component inside the router-view default slot', async () => {
        // Without stubbing router-view we exercise the v-slot binding that
        // keys the inner component on `route.path`.
        const Routed = { template: '<div class="routed-stub">hello</div>' }
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: Routed }],
        })
        await router.push('/')
        await router.isReady()
        const wrapper = mount(MainLayout, {
            global: {
                plugins: [createPinia(), i18n, router],
                stubs: {
                    AppBar: { template: '<div class="appbar-stub" />' },
                    AppSidebar: { template: '<div class="sidebar-stub" />' },
                    'v-main': { template: '<main><slot /></main>' },
                    'v-container': { template: '<div><slot /></div>' },
                },
            },
        })
        await flushPromises()
        expect(wrapper.find('.routed-stub').exists()).toBe(true)
    })
})
