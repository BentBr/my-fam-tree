import { createHead } from '@unhead/vue/client'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import DataPolicyView from '@/views/public/DataPolicyView.vue'

async function mountDp() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/data-policy', component: DataPolicyView }],
    })
    await router.push('/data-policy')
    await router.isReady()
    return mount(DataPolicyView, {
        global: { plugins: [createPinia(), i18n, router, createHead()] },
    })
}

describe('DataPolicyView', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    it('renders the eight-section policy shell', async () => {
        const w = await mountDp()
        expect(w.find('[data-testid="public-data-policy"]').exists()).toBe(true)
        const headings = w.findAll('.legal__heading').map((h) => h.text())
        expect(headings).toHaveLength(8)
    })

    it('renders the two auth cookies in the dedicated <li> block', async () => {
        const w = await mountDp()
        const items = w.findAll('.legal__cookies li').map((li) => li.text())
        expect(items).toHaveLength(2)
        expect(items[0]).toMatch(/access/i)
        expect(items[1]).toMatch(/refresh/i)
    })

    it('renders the five GDPR rights bullets', async () => {
        const w = await mountDp()
        const allLis = w.findAll('section.legal__section ul:not(.legal__cookies) li')
        // The Rights section is the only one using a regular <ul>; the
        // cookies section uses .legal__cookies which is excluded above.
        expect(allLis.length).toBeGreaterThanOrEqual(5)
    })

    it('interpolates the contact email through the {email} slot (no bare @ in catalog)', async () => {
        const w = await mountDp()
        // The contact + rights paragraphs both reference the email; if
        // the i18n compiler had choked on a literal `@` the render would
        // have aborted. A successful mount + a containing text block is
        // enough proof the slot interpolation is wired correctly.
        expect(w.text()).toContain('hello@my-fam-tree.eu')
    })
})
