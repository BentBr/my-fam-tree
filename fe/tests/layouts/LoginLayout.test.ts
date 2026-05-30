import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { createRouter, createMemoryHistory } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import LoginLayout from '@/layouts/LoginLayout.vue'

function makeRouter() {
    return createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/', component: { template: '<div>x</div>' } }],
    })
}

describe('LoginLayout', () => {
    it('mounts the unified AppBar above the sign-in card', async () => {
        const router = makeRouter()
        await router.push('/')
        await router.isReady()
        const wrapper = mount(LoginLayout, {
            global: {
                plugins: [createPinia(), i18n, router],
                stubs: {
                    'v-main': { template: '<main><slot /></main>' },
                    'v-container': { template: '<div><slot /></div>' },
                    'router-view': { template: '<div />' },
                    AppBar: { template: '<div class="appbar-stub" data-testid="app-bar" />' },
                },
            },
        })
        // The AppBar carries the brand + theme + language + account controls
        // for every layout. Its presence here proves LoginLayout no longer
        // hand-rolls its own header (the language switcher used to live
        // inline; it's now an atom inside the shared chrome).
        expect(wrapper.find('[data-testid="app-bar"]').exists()).toBe(true)
    })

    it('renders the routed component inside the router-view default slot', async () => {
        // Without stubbing router-view we exercise the v-slot binding that
        // keys the inner component on `route.fullPath` (covers LoginLayout.vue
        // template lines 18-20).
        const Routed = { template: '<div class="routed-stub">hello</div>' }
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: Routed }],
        })
        await router.push('/')
        await router.isReady()
        const wrapper = mount(LoginLayout, {
            global: {
                plugins: [createPinia(), i18n, router],
                stubs: {
                    'v-main': { template: '<main><slot /></main>' },
                    'v-container': { template: '<div><slot /></div>' },
                    AppBar: { template: '<div class="appbar-stub" />' },
                },
            },
        })
        expect(wrapper.find('.routed-stub').exists()).toBe(true)
    })
})
