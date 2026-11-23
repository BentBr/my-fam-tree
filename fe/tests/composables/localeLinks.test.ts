import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn() } }))
const patchSpy = vi.fn()
vi.mock('@/api/hooks/users', () => ({
    useUpdateMe: () => ({ mutate: patchSpy }),
}))

import { useLocaleSwitch } from '@/composables/useLocaleSwitch'
import { useLocalizedPath } from '@/composables/useLocalizedPath'
import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'
import { memoryStorage } from '../helpers/memory-storage'

function stubStorage(): void {
    vi.stubGlobal('localStorage', memoryStorage())
}

function makeRouter(start = '/en'): Router {
    const LANG = ':lang(en|de)'
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: `/${LANG}`, name: 'home', component: { template: '<div />' } },
            { path: `/${LANG}/imprint`, name: 'imprint', component: { template: '<div />' } },
            { path: `/${LANG}/auth/sign-in`, name: 'sign-in', component: { template: '<div />' } },
        ],
    })
    void router.push(start)
    return router
}

/** Mount a throwaway host that calls a composable inside `setup`. */
async function withComposable<T>(router: Router, fn: () => T): Promise<{ result: T; router: Router }> {
    await router.isReady()
    let result!: T
    const Host = defineComponent({
        setup() {
            result = fn()
            return () => h('div')
        },
    })
    const qc = new QueryClient({ defaultOptions: { queries: { retry: 0 }, mutations: { retry: 0 } } })
    mount(Host, { global: { plugins: [createPinia(), i18n, router, [VueQueryPlugin, { queryClient: qc }]] } })
    return { result, router }
}

describe('useLocalizedPath', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        stubStorage()
    })

    it('prefixes a path with the active locale; bare root collapses to just the prefix', async () => {
        const { result: localized } = await withComposable(makeRouter(), () => useLocalizedPath())
        useLocaleStore().set('de')
        expect(localized('/imprint')).toBe('/de/imprint')
        expect(localized('/')).toBe('/de')
        useLocaleStore().set('en')
        expect(localized('/auth/sign-in')).toBe('/en/auth/sign-in')
    })
})

describe('useLocaleSwitch', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        stubStorage()
        patchSpy.mockReset()
    })

    it('flips the store AND swaps the URL prefix in place', async () => {
        const { result: pick, router } = await withComposable(makeRouter('/en/imprint'), () => useLocaleSwitch())
        pick('de')
        expect(useLocaleStore().locale).toBe('de')
        await flushPromises()
        // Suffix preserved, only the locale segment swapped.
        expect(router.currentRoute.value.path).toBe('/de/imprint')
    })

    it('writes the preference to /users/me only when authenticated', async () => {
        const { result: pick } = await withComposable(makeRouter(), () => useLocaleSwitch())
        pick('de')
        expect(patchSpy).not.toHaveBeenCalled()

        useAuthStore().applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'de',
            edge_style: 'curved',
            families: [],
        })
        pick('en')
        expect(patchSpy).toHaveBeenCalledWith({ locale: 'en' })
    })

    it('is a no-op when picking the already-active locale', async () => {
        const { result: pick, router } = await withComposable(makeRouter('/en'), () => useLocaleSwitch())
        const before = router.currentRoute.value.fullPath
        pick('en')
        await flushPromises()
        expect(router.currentRoute.value.fullPath).toBe(before)
        expect(patchSpy).not.toHaveBeenCalled()
    })
})
