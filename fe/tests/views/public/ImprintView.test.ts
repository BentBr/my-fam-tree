import { createHead } from '@unhead/vue/client'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import ImprintView from '@/views/public/ImprintView.vue'

async function mountImprint() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/imprint', component: ImprintView }],
    })
    await router.push('/imprint')
    await router.isReady()
    return mount(ImprintView, {
        global: { plugins: [createPinia(), i18n, router, createHead()] },
    })
}

describe('ImprintView', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    afterEach(() => {
        vi.useRealTimers()
    })

    it('renders the imprint shell with the email contact', async () => {
        const w = await mountImprint()
        expect(w.find('[data-testid="public-imprint"]').exists()).toBe(true)
        const mail = w.find('a[href^="mailto:"]')
        expect(mail.exists()).toBe(true)
        expect(mail.attributes('href')).toContain('@')
        expect(mail.attributes('href')).toContain('my-fam-tree.eu')
    })

    it('renders each of the four section headings', async () => {
        const w = await mountImprint()
        // i18n keys: contact, responsible, disclaimer, linkLiability.
        const headings = w.findAll('.legal__heading').map((h) => h.text())
        expect(headings).toHaveLength(4)
    })

    it('links slothlike.eu below the responsible person', async () => {
        const w = await mountImprint()
        const link = w.find('[data-testid="imprint-company-link"]')
        expect(link.exists()).toBe(true)
        expect(link.text()).toBe('slothlike.eu')
        expect(link.attributes('href')).toBe('https://slothlike.eu')
        expect(link.attributes('target')).toBe('_blank')
        expect(link.element.parentElement?.textContent).toContain('Bent Brüggemann')
    })

    it('renders the current year in the footer note, not a hard-coded one', async () => {
        vi.useFakeTimers({ toFake: ['Date'] })
        vi.setSystemTime(new Date('2031-06-15T12:00:00Z'))
        const w = await mountImprint()
        expect(w.find('.legal__footer').text()).toContain('© 2031 Slothlike.')
    })
})
