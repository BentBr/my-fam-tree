import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

import PublicFooter from '@/components/layout/PublicFooter.vue'
import { i18n } from '@/i18n'

async function mountFooter() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: { template: '<div />' } },
            { path: '/imprint', component: { template: '<div />' } },
            { path: '/data-policy', component: { template: '<div />' } },
        ],
    })
    await router.push('/')
    await router.isReady()
    return mount(PublicFooter, { global: { plugins: [createPinia(), i18n, router] } })
}

describe('PublicFooter', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    it('links imprint + data-policy + shows the locale label', async () => {
        const w = await mountFooter()
        const imprint = w.find('[data-testid="footer-imprint"]')
        const dp = w.find('[data-testid="footer-data-policy"]')
        expect(imprint.exists()).toBe(true)
        expect(dp.exists()).toBe(true)
        expect(imprint.attributes('href')).toBe('/imprint')
        expect(dp.attributes('href')).toBe('/data-policy')
        expect(w.text()).toContain('English')
    })
})
