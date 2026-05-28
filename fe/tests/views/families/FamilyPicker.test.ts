import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

vi.mock('@/api/client', () => ({
    client: {
        // useMyFamilies (added Phase 5 Task 23) calls client.GET('/families/me');
        // stub it to return an empty list so the picker falls back to role-only
        // subtitles in the default-render test.
        GET: vi.fn(async () => ({ data: { data: { families: [] } } })),
        POST: vi.fn(),
    },
}))

import { i18n } from '@/i18n'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import FamilyPicker from '@/views/families/FamilyPicker.vue'

function makeRouter(): Router {
    return createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: { template: '<div />' } },
            { path: '/tree', component: { template: '<div />' } },
            { path: '/families/create', component: { template: '<div />' } },
        ],
    })
}

async function mountView() {
    const router = makeRouter()
    await router.push('/')
    await router.isReady()
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: 0 } } })
    return mount(FamilyPicker, {
        global: {
            plugins: [i18n, router, [VueQueryPlugin, { queryClient }]],
            stubs: {
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-list': { template: '<div><slot /></div>' },
                'v-list-item': {
                    template:
                        '<button class="item" :data-testid="$attrs[\'data-testid\']" :data-subtitle="subtitle" @click="$emit(\'click\')">{{ title }}</button>',
                    props: ['title', 'subtitle'],
                    emits: ['click'],
                },
                'v-btn': { template: '<a class="btn-link"><slot /></a>', props: ['to', 'variant'] },
            },
        },
    })
}

describe('FamilyPicker', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
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
    })

    it('renders one item per family', async () => {
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
        const w = await mountView()
        expect(w.findAll('.item')).toHaveLength(2)
    })

    it('clicking an item sets active family and redirects to /tree (not the old /health default)', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const router = makeRouter()
        await router.push('/')
        await router.isReady()
        const queryClient = new QueryClient({ defaultOptions: { queries: { retry: 0 } } })
        const w = mount(FamilyPicker, {
            global: {
                plugins: [i18n, router, [VueQueryPlugin, { queryClient }]],
                stubs: {
                    'v-card': { template: '<div><slot /></div>' },
                    'v-card-title': { template: '<div><slot /></div>' },
                    'v-list': { template: '<div><slot /></div>' },
                    'v-list-item': {
                        template:
                            '<button class="item" :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\')">{{ title }}</button>',
                        props: ['title', 'subtitle'],
                        emits: ['click'],
                    },
                    'v-btn': { template: '<a class="btn-link"><slot /></a>', props: ['to', 'variant'] },
                },
            },
        })
        await w.find('.item').trigger('click')
        await flushPromises()
        const family = useActiveFamilyStore()
        expect(family.activeFamilyId).toBe('f-1')
        expect(router.currentRoute.value.path).toBe('/tree')
    })

    it('shows the role + created-date subtitle ONLY for duplicate names', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [
                { id: 'f-1', name: 'Müller', role: 'owner' },
                { id: 'f-2', name: 'Peters', role: 'owner' },
                { id: 'f-3', name: 'Peters', role: 'user' },
            ],
        } as never)
        // Backend supplies created_at on /families/me; FE shows it only when
        // the name actually repeats. Stub the live response with two Peters
        // entries that have created_at, one unique Müller.
        const { client } = await import('@/api/client')
        ;(client.GET as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
            data: {
                data: {
                    families: [
                        { id: 'f-1', name: 'Müller', role: 'owner', created_at: '2024-03-12T10:00:00Z' },
                        { id: 'f-2', name: 'Peters', role: 'owner', created_at: '2026-05-01T12:00:00Z' },
                        { id: 'f-3', name: 'Peters', role: 'user', created_at: '2025-09-20T08:00:00Z' },
                    ],
                },
            },
        })

        const w = await mountView()
        await flushPromises()
        const sub = (id: string): string | null =>
            w.find(`[data-testid="pick-${id}"]`).attributes('data-subtitle') ?? null
        // Unique name → role only (no date).
        expect(sub('f-1')).toBe('owner')
        // Both Peters entries (same name) → role + their own created date.
        expect(sub('f-2')).toMatch(/owner.*May.*2026|owner.*2026.*May/)
        expect(sub('f-3')).toMatch(/user.*Sept?.*2025|user.*2025.*Sept?/)
    })
})
