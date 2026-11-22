import { createHead } from '@unhead/vue/server'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, type FunctionalComponent, h, nextTick } from 'vue'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { useRouteMeta } from '@/composables/useRouteMeta'
import { i18n } from '@/i18n'
import { useLocaleStore } from '@/stores/locale'
import type { SupportedLocale } from '@/stores/locale'

// A throwaway route component — `useRouteMeta` reads `route.name`, so
// the meaningful state is the route record, not what the page renders.
const Blank: FunctionalComponent = () => h('div')

// Locale-prefixed like the real route table — `usePageMeta` derives the
// canonical + hreflang URLs from the `/en`|`/de` prefix of the live path.
function makeRouter(): Router {
    const LANG = ':lang(en|de)'
    return createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: `/${LANG}`, name: 'home', component: Blank },
            { path: `/${LANG}/imprint`, name: 'imprint', component: Blank },
            { path: `/${LANG}/data-policy`, name: 'data-policy', component: Blank },
            { path: `/${LANG}/tree`, name: 'tree', component: Blank },
            { path: `/${LANG}/nowhere`, name: 'nowhere', component: Blank },
        ],
    })
}

/**
 * Mount a tiny host that wires `useRouteMeta()` against a server head,
 * navigate to `path`, and return the rendered head payload so a test can
 * assert on the resolved `<title>` + `<meta>` + `<link>` strings without
 * fighting DOM-flush timing.
 */
async function renderHeadAt(path: string, locale: SupportedLocale = 'en') {
    const router = makeRouter()
    await router.push(path)
    await router.isReady()
    const head = createHead()
    const Host = defineComponent({
        setup() {
            useRouteMeta()
            return () => h('div')
        },
    })
    // ONE pinia for both the mounted component and the harness assertions —
    // a separate `createPinia()` in plugins would give the composable a
    // different store instance than the one we `set()` below.
    const pinia = createPinia()
    setActivePinia(pinia)
    const wrapper = mount(Host, { global: { plugins: [pinia, i18n, router, head] } })
    // The store only drives vue-i18n once bound — main.ts does that at boot;
    // the harness must do it explicitly or `set()` never reaches `t()`.
    const localeStore = useLocaleStore()
    localeStore.bindToI18n(i18n)
    // Set the locale AFTER mount so the reactive getters have something
    // to recompute against in the locale-switch test.
    localeStore.set(locale)
    await nextTick()
    const payload = head.render()
    return { payload, router, head, wrapper }
}

describe('usePageMeta via useRouteMeta', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })
    afterEach(() => {
        useLocaleStore().set('en')
    })

    it('emits the full SEO + OG/Twitter block for the indexable home route', async () => {
        const { payload } = await renderHeadAt('/en')
        const { headTags } = payload

        // Title carries the page segment + brand suffix.
        expect(payload.bodyTags).toBeDefined()
        expect(headTags).toContain('<title>Map your family. Share what matters. · My Family Tree</title>')
        expect(headTags).toContain('name="description"')
        expect(headTags).toContain('content="index, follow"')

        // The complete OpenGraph set the spec mandates.
        expect(headTags).toContain('property="og:type"')
        expect(headTags).toContain('property="og:site_name"')
        expect(headTags).toContain('property="og:title"')
        expect(headTags).toContain('property="og:description"')
        expect(headTags).toContain('property="og:image"')
        expect(headTags).toContain('https://my-fam-tree.eu/brand/og-1200x630.png')
        expect(headTags).toContain('property="og:url"')
        expect(headTags).toContain('property="og:locale"')
        expect(headTags).toContain('content="en_US"')
        expect(headTags).toContain('property="og:locale:alternate"')
        expect(headTags).toContain('content="de_DE"')

        // Twitter summary_large_image card.
        expect(headTags).toContain('name="twitter:card"')
        expect(headTags).toContain('content="summary_large_image"')

        // Canonical + hreflang alternates point at the locale-PREFIXED
        // variants vite-ssg prerenders (home is the only hreflang page).
        expect(headTags).toContain('rel="canonical"')
        expect(headTags).toContain('href="https://my-fam-tree.eu/en"')
        expect(headTags).toContain('hreflang="de"')
        expect(headTags).toContain('href="https://my-fam-tree.eu/de"')
        expect(headTags).toContain('hreflang="x-default"')
        expect(headTags).not.toContain('?lang=')
    })

    it('emits ONLY a minimal title + robots noindex for the legal pages', async () => {
        for (const path of ['/en/imprint', '/en/data-policy']) {
            const { payload } = await renderHeadAt(path)
            const { headTags } = payload
            expect(headTags).toContain('content="noindex, nofollow"')
            // No description, no OG, no canonical, no hreflang on legal pages.
            expect(headTags).not.toContain('name="description"')
            expect(headTags).not.toContain('property="og:')
            expect(headTags).not.toContain('rel="canonical"')
            expect(headTags).not.toContain('hreflang')
            // Still a branded title.
            expect(headTags).toContain('· My Family Tree</title>')
        }
    })

    it('gives an authenticated route a translated title + description + OG (no hreflang)', async () => {
        const { payload } = await renderHeadAt('/en/tree')
        const { headTags } = payload
        expect(headTags).toContain('<title>Family tree · My Family Tree</title>')
        expect(headTags).toContain('name="description"')
        expect(headTags).toContain('property="og:title"')
        // Self-canonical to the live, locale-prefixed route path.
        expect(headTags).toContain('rel="canonical"')
        expect(headTags).toContain('href="https://my-fam-tree.eu/en/tree"')
        // Authenticated pages are not crawled — no hreflang alternates.
        expect(headTags).not.toContain('hreflang')
    })

    it('falls back to the default descriptor for an unmapped route', async () => {
        const { payload } = await renderHeadAt('/en/nowhere')
        expect(payload.headTags).toContain('<title>My Family Tree · My Family Tree</title>')
        expect(payload.headTags).toContain('name="description"')
    })

    it('swaps the resolved tags when the locale switches to German', async () => {
        const { payload } = await renderHeadAt('/de', 'de')
        const { headTags } = payload
        // German title + description from the de catalogue.
        expect(headTags).toContain('<title>Erfasse Deine Familie. Teile was zählt. · My Family Tree</title>')
        expect(headTags).toContain('content="de_DE"') // og:locale flips to German
        // and the alternate flips to English.
        expect(headTags).toContain('content="en_US"')
        expect(payload.htmlAttrs).toContain('lang="de"')
    })
})
