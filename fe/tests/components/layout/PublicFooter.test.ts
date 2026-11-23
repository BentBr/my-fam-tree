import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

import PublicFooter from '@/components/layout/PublicFooter.vue'
import { i18n } from '@/i18n'

async function mountFooter() {
    // Locale-prefixed like the real route table — the footer's legal
    // links carry the active locale (they land in prerendered HTML).
    const LANG = ':lang(en|de)'
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: `/${LANG}`, component: { template: '<div />' } },
            { path: `/${LANG}/imprint`, component: { template: '<div />' } },
            { path: `/${LANG}/data-policy`, component: { template: '<div />' } },
        ],
    })
    await router.push('/en')
    await router.isReady()
    return mount(PublicFooter, { global: { plugins: [createPinia(), i18n, router] } })
}

describe('PublicFooter', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    afterEach(() => {
        vi.useRealTimers()
    })

    it('links the LOCALE-PREFIXED imprint + data-policy + shows the locale label', async () => {
        const w = await mountFooter()
        const imprint = w.find('[data-testid="footer-imprint"]')
        const dp = w.find('[data-testid="footer-data-policy"]')
        expect(imprint.exists()).toBe(true)
        expect(dp.exists()).toBe(true)
        expect(imprint.attributes('href')).toBe('/en/imprint')
        expect(dp.attributes('href')).toBe('/en/data-policy')
        expect(w.text()).toContain('English')
    })

    it('links the Slothlike copyright holder to slothlike.eu in a new tab', async () => {
        const w = await mountFooter()
        const company = w.find('[data-testid="footer-company"]')
        expect(company.exists()).toBe(true)
        expect(company.text()).toBe('Slothlike')
        expect(company.attributes('href')).toBe('https://slothlike.eu')
        expect(company.attributes('target')).toBe('_blank')
        expect(company.attributes('rel')).toContain('noopener')
    })

    it('renders the current year in the copyright, not a hard-coded one', async () => {
        vi.useFakeTimers({ toFake: ['Date'] })
        vi.setSystemTime(new Date('2031-06-15T12:00:00Z'))
        const w = await mountFooter()
        expect(w.find('[data-testid="public-footer-tagline"]').text()).toContain('© 2031 Slothlike.')
    })
})
