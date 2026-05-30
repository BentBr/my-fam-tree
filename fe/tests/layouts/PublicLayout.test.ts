import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import PublicLayout from '@/layouts/PublicLayout.vue'

describe('PublicLayout', () => {
    it('mounts AppBar + router-view + PublicFooter', async () => {
        const Routed = { template: '<div class="routed">hi</div>' }
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: Routed }],
        })
        await router.push('/')
        await router.isReady()
        const wrapper = mount(PublicLayout, {
            global: {
                plugins: [createPinia(), i18n, router],
                stubs: {
                    AppBar: { template: '<div class="appbar-stub" />' },
                    PublicFooter: { template: '<footer class="footer-stub" />' },
                    'v-main': { template: '<main><slot /></main>' },
                },
            },
        })
        expect(wrapper.find('.appbar-stub').exists()).toBe(true)
        expect(wrapper.find('.footer-stub').exists()).toBe(true)
        expect(wrapper.find('.routed').exists()).toBe(true)
    })
})
