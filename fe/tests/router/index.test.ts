/**
 * Router guards + locale normalization. Under vite-ssg the router is
 * created by the framework, so the module exports `routes` +
 * `registerGuards` instead of a singleton — each test builds a fresh
 * memory-history router from those, seeding the auth / activeFamily
 * stores beforehand.
 *
 * Every route is locale-prefixed; bare paths resolve through the
 * catch-all normalizer (`/tree` → `/en/tree` for the mocked en-US
 * browser), so pushes below keep using bare paths exactly like view
 * code does, and assertions expect the prefixed result.
 */
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Mock } from 'vitest'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { client } from '@/api/client'
import { registerGuards, routes } from '@/router'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import type { FamilyId } from '@/types/brand'
import { memoryStorage } from '../helpers/memory-storage'

function mockStorage(): void {
    vi.stubGlobal('localStorage', memoryStorage())
}

async function makeRouter(start = '/auth/sign-in'): Promise<Router> {
    const router = createRouter({ history: createMemoryHistory(), routes })
    registerGuards(router)
    await router.replace(start)
    await router.isReady()
    return router
}

describe('router guards', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        mockStorage()
        ;(client.GET as Mock).mockReset()
    })

    it('anonymous user gets bounced to sign-in when navigating to a protected route', async () => {
        const router = await makeRouter()
        await router.push('/tree')
        expect(router.currentRoute.value.path).toBe('/en/auth/sign-in')
    })

    it('anonymous user can reach /auth/* without bouncing', async () => {
        const router = await makeRouter('/en')
        await router.push('/auth/sign-in')
        expect(router.currentRoute.value.path).toBe('/en/auth/sign-in')
    })

    it('anonymous user can reach /invite/* without bouncing (token stash flow)', async () => {
        const router = await makeRouter()
        await router.push('/invite/accept')
        expect(router.currentRoute.value.path).toBe('/en/invite/accept')
    })

    it('authenticated user lands on /health from sign-in instead of staying', async () => {
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        // Force a non-sign-in starting route so the push fires a real
        // navigation (same-route pushes are a no-op).
        await router.replace('/health')
        await router.push('/auth/sign-in')
        expect(router.currentRoute.value.path).toBe('/en/health')
    })

    it('authenticated user without families is redirected to /families/create', async () => {
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [],
        })
        await router.push('/tree')
        expect(router.currentRoute.value.path).toBe('/en/families/create')
    })

    it('authenticated user with exactly one family auto-selects it and proceeds', async () => {
        // T7: single-family users skip the picker entirely — there's nothing
        // to choose between, so the guard sets the family active in-place
        // and continues the navigation rather than bouncing through /families/pick.
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        await router.push('/tree')
        expect(router.currentRoute.value.path).toBe('/en/tree')
        const family = useActiveFamilyStore()
        expect(family.activeFamilyId).toBe('f-1')
    })

    it('authenticated user with multiple families and none active is redirected to /families/pick', async () => {
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [
                { id: 'f-1', name: 'F1', role: 'owner' },
                { id: 'f-2', name: 'F2', role: 'user' },
            ],
        } as never)
        await router.push('/tree')
        expect(router.currentRoute.value.path).toBe('/en/families/pick')
    })

    it('authenticated user with an active family proceeds to the target route', async () => {
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        await router.push('/health')
        expect(router.currentRoute.value.path).toBe('/en/health')
    })

    it('authenticated user can reach /families/* without family-selection enforcement', async () => {
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        await router.push('/families/pick')
        expect(router.currentRoute.value.path).toBe('/en/families/pick')
    })

    it('anonymous user is bounced from /health (authed status page) consistently with its meta', async () => {
        // Regression for the meta/behaviour mismatch: /health is an
        // authenticated page (main chrome, post-login landing). Its meta
        // now says `requiresAuth: true` and the meta-driven guard bounces
        // anonymous visitors — the two agree.
        const router = await makeRouter()
        await router.push('/health')
        expect(router.currentRoute.value.path).toBe('/en/auth/sign-in')
    })

    it('/ resolves to the localized public home for anonymous visitors', async () => {
        // The bare root goes through the locale normalizer; the en-US
        // browser lands on `/en`, which is public — no sign-in bounce.
        const router = await makeRouter('/en')
        await router.push('/')
        expect(router.currentRoute.value.path).toBe('/en')
        expect(router.currentRoute.value.meta.public).toBe(true)
    })

    it('clears a stale activeFamilyId that is no longer in the membership list', async () => {
        // Seed the auth payload with one family, then forcibly set a DIFFERENT
        // family id active — mimicking localStorage carrying a stale id from a
        // previous identity. The guard should detect the mismatch, clear the
        // active id, and (with only one available family) auto-select it.
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const family = useActiveFamilyStore()
        // Bypass setActive's membership check by writing the ref directly.
        family.activeFamilyId = 'f-stale' as FamilyId
        await router.push('/health')
        // Stale id was cleared then the sole-family auto-select kicked in.
        expect(family.activeFamilyId).toBe('f-1')
        expect(router.currentRoute.value.path).toBe('/en/health')
    })

    it('admin role gate lets owners through to /admin/audit', async () => {
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        await router.push('/admin/audit')
        expect(router.currentRoute.value.path).toBe('/en/admin/audit')
    })

    it('admin role gate bounces plain users away from /admin/* to /tree', async () => {
        const router = await makeRouter()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            edge_style: 'curved',
            families: [{ id: 'f-1', name: 'F1', role: 'user' }],
        } as never)
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        await router.push('/admin/audit')
        expect(router.currentRoute.value.path).toBe('/en/tree')
    })
})

describe('locale normalization', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'de-DE' })
        mockStorage()
        ;(client.GET as Mock).mockReset()
    })

    it('redirects the bare root to the browser-default locale home', async () => {
        const router = await makeRouter('/de')
        await router.push('/')
        expect(router.currentRoute.value.path).toBe('/de')
    })

    it('prefixes a bare deep path and preserves query + hash (magic-link shape)', async () => {
        const router = await makeRouter('/de')
        await router.push('/auth/consume?token=tok-1#frag')
        expect(router.currentRoute.value.fullPath).toBe('/de/auth/consume?token=tok-1#frag')
    })

    it('keeps OLD anonymous backend-link shapes working with their token intact', async () => {
        // Emails sent BEFORE the locale-prefix migration carry bare paths.
        // Each must survive the normalizer with route + token untouched —
        // an invite or magic link in an old inbox stays clickable.
        const oldLinks = [
            ['/invite/accept?token=inv-1', '/de/invite/accept?token=inv-1', 'invite-accept'],
            ['/auth/consume?token=ml-1', '/de/auth/consume?token=ml-1', 'consume'],
        ] as const
        for (const [old, expected, name] of oldLinks) {
            const router = await makeRouter('/de')
            await router.push(old)
            expect(router.currentRoute.value.fullPath).toBe(expected)
            expect(router.currentRoute.value.name).toBe(name)
        }
    })

    it('keeps OLD session-bound backend-link shapes working with their token intact', async () => {
        // Email-change + owner-transfer links require a session; with one
        // active, the bare path normalizes and the token survives.
        const oldLinks = [
            [
                '/account/email-change/consume?token=ec-1',
                '/de/account/email-change/consume?token=ec-1',
                'email-change-consume',
            ],
            [
                '/account/owner-transfer/confirm?token=ot-1',
                '/de/account/owner-transfer/confirm?token=ot-1',
                'owner-transfer-confirm',
            ],
        ] as const
        for (const [old, expected, name] of oldLinks) {
            const router = await makeRouter('/de')
            useAuthStore().applyClaimsPayload({
                user_id: 'u',
                email: 'a@b',
                locale: 'de',
                edge_style: 'curved',
                families: [{ id: 'f-1', name: 'F', role: 'owner' }],
            } as never)
            await router.push(old)
            expect(router.currentRoute.value.fullPath).toBe(expected)
            expect(router.currentRoute.value.name).toBe(name)
            setActivePinia(createPinia())
        }
    })

    it('prefers the stored locale over the browser language for bare paths', async () => {
        localStorage.setItem('my-fam-tree:locale', 'en')
        const router = await makeRouter('/en')
        await router.push('/imprint')
        expect(router.currentRoute.value.path).toBe('/en/imprint')
    })

    it('redirects an unknown locale segment to the default locale with the same suffix', async () => {
        const router = await makeRouter('/de')
        await router.push('/fr/imprint')
        expect(router.currentRoute.value.path).toBe('/de/imprint')
    })

    it('falls back to the locale home for an unknown page under a known locale', async () => {
        const router = await makeRouter('/de')
        await router.push('/en/does-not-exist')
        expect(router.currentRoute.value.path).toBe('/en')
    })

    it('seeds the locale store from the URL prefix on every navigation', async () => {
        const router = await makeRouter('/de')
        const { useLocaleStore } = await import('@/stores/locale')
        const locale = useLocaleStore()
        await router.push('/en/imprint')
        expect(locale.locale).toBe('en')
        await router.push('/de/imprint')
        expect(locale.locale).toBe('de')
    })

    it('enforces the account locale over the URL for a signed-in user', async () => {
        // Hydrate succeeds with a German-preference user while the visitor
        // sits on an /en URL: the account locale is AUTHORITATIVE, so the
        // prefix is forced to `/de` — and STAYS enforced on later navigations
        // (an /en link is redirected back to /de). The router is built while
        // the session is still anonymous (the default mock rejects); the
        // claims mock is installed AFTER, so sign-in happens on the push.
        const router = await makeRouter('/en')
        ;(client.GET as Mock).mockResolvedValue({
            data: {
                data: {
                    user_id: 'u',
                    email: 'a@b',
                    locale: 'de',
                    edge_style: 'curved',
                    families: [{ id: 'f-1', name: 'F', role: 'owner' }],
                },
            },
            error: undefined,
        })
        await router.push('/en/health')
        expect(router.currentRoute.value.path).toBe('/de/health')
        // Still enforced afterwards: an explicit /en link bounces back to /de.
        await router.push('/en/account')
        expect(router.currentRoute.value.path).toBe('/de/account')
    })
})
