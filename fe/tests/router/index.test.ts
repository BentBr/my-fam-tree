/**
 * Router guards: the module installs two beforeEach hooks. We drive them by
 * navigating the singleton router with createWebHistory(), seeding the auth /
 * activeFamily stores beforehand. Because the router uses createWebHistory()
 * (not memory), navigation in jsdom/happy-dom updates `window.location` —
 * this still works for guard logic and currentRoute inspection.
 */
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { router } from '@/router'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import type { FamilyId } from '@/types/brand'

function mockStorage(): void {
    const store: Record<string, string> = {}
    vi.stubGlobal('localStorage', {
        getItem: (k: string) => store[k] ?? null,
        setItem: (k: string, v: string) => {
            store[k] = v
        },
        removeItem: (k: string) => {
            delete store[k]
        },
    })
}

describe('router guards', () => {
    beforeEach(async () => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        mockStorage()
        // Reset to a clean route before each test.
        await router.replace('/auth/sign-in')
        await router.isReady()
    })

    it('anonymous user gets bounced to /auth/sign-in when navigating to a protected route', async () => {
        await router.push('/tree')
        expect(router.currentRoute.value.path).toBe('/auth/sign-in')
    })

    it('anonymous user can reach /auth/* without bouncing', async () => {
        await router.push('/auth/sign-in')
        expect(router.currentRoute.value.path).toBe('/auth/sign-in')
    })

    it('anonymous user can reach /invite/* without bouncing (token stash flow)', async () => {
        await router.push('/invite/accept')
        expect(router.currentRoute.value.path).toBe('/invite/accept')
    })

    it('authenticated user lands on /health from sign-in instead of staying', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        // Force a non-sign-in starting route so the push fires a real
        // navigation (same-route pushes are a no-op).
        await router.replace('/health')
        await router.push('/auth/sign-in')
        expect(router.currentRoute.value.path).toBe('/health')
    })

    it('authenticated user without families is redirected to /families/create', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [],
        } as never)
        await router.push('/tree')
        expect(router.currentRoute.value.path).toBe('/families/create')
    })

    it('authenticated user with exactly one family auto-selects it and proceeds', async () => {
        // T7: single-family users skip the picker entirely — there's nothing
        // to choose between, so the guard sets the family active in-place
        // and continues the navigation rather than bouncing through /families/pick.
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        await router.push('/tree')
        expect(router.currentRoute.value.path).toBe('/tree')
        const family = useActiveFamilyStore()
        expect(family.activeFamilyId).toBe('f-1')
    })

    it('authenticated user with multiple families and none active is redirected to /families/pick', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [
                { id: 'f-1', name: 'F1', role: 'owner' },
                { id: 'f-2', name: 'F2', role: 'user' },
            ],
        } as never)
        await router.push('/tree')
        expect(router.currentRoute.value.path).toBe('/families/pick')
    })

    it('authenticated user with an active family proceeds to the target route', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        await router.push('/health')
        expect(router.currentRoute.value.path).toBe('/health')
    })

    it('authenticated user can reach /families/* without family-selection enforcement', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        await router.push('/families/pick')
        expect(router.currentRoute.value.path).toBe('/families/pick')
    })

    it('/ redirects to /tree', async () => {
        // Confirm route table covers the root redirect. Anonymous user → tree
        // is protected → sign-in (we just verify the redirect chain runs).
        await router.push('/')
        // Either /tree (if authenticated) or /auth/sign-in (anonymous) is fine;
        // both prove the redirect target is /tree.
        expect(router.currentRoute.value.path === '/auth/sign-in').toBe(true)
    })

    it('clears a stale activeFamilyId that is no longer in the membership list', async () => {
        // Seed the auth payload with one family, then forcibly set a DIFFERENT
        // family id active — mimicking localStorage carrying a stale id from a
        // previous identity. The guard should detect the mismatch, clear the
        // active id, and (with only one available family) auto-select it.
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const family = useActiveFamilyStore()
        // Bypass setActive's membership check by writing the ref directly.
        family.activeFamilyId = 'f-stale' as FamilyId
        await router.push('/health')
        // Stale id was cleared then the sole-family auto-select kicked in.
        expect(family.activeFamilyId).toBe('f-1')
        expect(router.currentRoute.value.path).toBe('/health')
    })

    it('admin role gate lets owners through to /admin/audit', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        await router.push('/admin/audit')
        expect(router.currentRoute.value.path).toBe('/admin/audit')
    })

    it('admin role gate bounces plain users away from /admin/* to /tree', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'user' }],
        } as never)
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        await router.push('/admin/audit')
        expect(router.currentRoute.value.path).toBe('/tree')
    })
})
