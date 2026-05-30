import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

import BrandLogo from '@/components/common/BrandLogo.vue'

async function mountLogo(props: Partial<{ size: 'sm' | 'md' | 'lg'; to: string | null }> = {}) {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/', component: { template: '<div />' } }],
    })
    await router.push('/')
    await router.isReady()
    return mount(BrandLogo, { props, global: { plugins: [router] } })
}

describe('BrandLogo', () => {
    it('renders the brand wordmark + subline at default md size', async () => {
        const w = await mountLogo()
        expect(w.text()).toContain('My Family Tree')
        expect(w.text()).toContain('by Slothlike')
        const img = w.find('img')
        expect(img.attributes('width')).toBe('36')
    })

    it('omits the wordmark at sm size (rail-mode lockup)', async () => {
        const w = await mountLogo({ size: 'sm' })
        expect(w.text()).not.toContain('My Family Tree')
        const img = w.find('img')
        expect(img.attributes('width')).toBe('24')
    })

    it('scales the icon up at lg size', async () => {
        const w = await mountLogo({ size: 'lg' })
        const img = w.find('img')
        expect(img.attributes('width')).toBe('48')
    })

    it('renders as a plain <div> when no `to` is given', async () => {
        const w = await mountLogo()
        expect(w.element.tagName.toLowerCase()).toBe('div')
    })

    it('renders as a RouterLink when `to` is set', async () => {
        const w = await mountLogo({ to: '/' })
        const link = w.find('a')
        expect(link.exists()).toBe(true)
        expect(link.classes()).toContain('brand-lockup--linked')
    })
})
