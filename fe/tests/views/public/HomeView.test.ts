import { createHead } from '@unhead/vue/client'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import HomeView from '@/views/public/HomeView.vue'

const stubs = {
    'v-btn': { template: '<button :data-testid="$attrs[\'data-testid\']"><slot /></button>' },
    'v-icon': { template: '<i :data-icon="$attrs.icon" />' },
    RouterLink: { template: '<a><slot /></a>' },
}

async function mountHome() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: HomeView },
            { path: '/auth/sign-in', component: { template: '<div />' } },
        ],
    })
    await router.push('/')
    await router.isReady()
    const head = createHead()
    return mount(HomeView, { global: { plugins: [createPinia(), i18n, router, head], stubs } })
}

describe('HomeView', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    it('renders the hero headline + lede + both CTAs', async () => {
        const w = await mountHome()
        expect(w.find('[data-testid="public-home"]').exists()).toBe(true)
        expect(w.find('[data-testid="home-cta-primary"]').exists()).toBe(true)
        expect(w.find('[data-testid="home-cta-secondary"]').exists()).toBe(true)
        expect(w.text()).toContain('Map your family')
    })

    it('renders the three feature cards', async () => {
        const w = await mountHome()
        // Each card renders the title under public.home.features.*.title.
        expect(w.text()).toContain('One tree, every relation.')
        expect(w.text()).toContain('Gentle reminders.')
        expect(w.text()).toContain('Yours alone.')
    })

    it('mounts the real screenshot image (not the placeholder caption)', async () => {
        const w = await mountHome()
        const screenshot = w.find('img[src="/brand/tree-example-960.webp"]')
        expect(screenshot.exists()).toBe(true)
    })

    it('renders the footer CTA button', async () => {
        const w = await mountHome()
        expect(w.find('[data-testid="home-cta-footer"]').exists()).toBe(true)
    })
})
