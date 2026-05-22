import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import FamilySwitcher from '@/components/layout/FamilySwitcher.vue'
import { i18n } from '@/i18n'
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

async function mountSwitcher() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: { template: '<div />' } },
            { path: '/families/create', component: { template: '<div />' } },
        ],
    })
    await router.push('/')
    await router.isReady()
    const w = mount(FamilySwitcher, {
        global: {
            plugins: [i18n, router],
            stubs: {
                VSelect: {
                    name: 'VSelectStub',
                    props: ['modelValue', 'items'],
                    emits: ['update:modelValue'],
                    template: '<div class="select-stub" :data-items="JSON.stringify(items)" />',
                },
            },
        },
    })
    return { router, w }
}

describe('FamilySwitcher', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        mockStorage()
    })

    it('renders a create-only switcher when the user has no families', async () => {
        const { w } = await mountSwitcher()
        // T7: the switcher is always visible; with zero families the only
        // entry is the "create new" sentinel — never disappear entirely.
        const stub = w.find('.select-stub')
        expect(stub.exists()).toBe(true)
        const items = JSON.parse(stub.attributes('data-items') ?? '[]') as Array<Record<string, unknown>>
        expect(items.length).toBe(1)
        expect(items[0]?.['value']).toBe('__create__')
    })

    it('renders items + divider + create-new sentinel when families are present', async () => {
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
        const { w } = await mountSwitcher()
        const items = JSON.parse(w.find('.select-stub').attributes('data-items') ?? '[]') as Array<
            Record<string, unknown>
        >
        expect(items.length).toBe(4) // 2 families + divider + create
        expect(items[2]?.['type']).toBe('divider')
        expect(items[3]?.['value']).toBe('__create__')
    })

    it('onChange with a family id sets the active family', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const { w } = await mountSwitcher()
        const family = useActiveFamilyStore()
        // Drive the select's @update:model-value directly via vm emit:
        await w.findComponent({ name: 'VSelectStub' }).vm.$emit('update:modelValue', 'f-1' as FamilyId)
        expect(family.activeFamilyId).toBe('f-1')
    })

    it('onChange with the create-sentinel pushes to /families/create', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const { w, router } = await mountSwitcher()
        await w.findComponent({ name: 'VSelectStub' }).vm.$emit('update:modelValue', '__create__')
        // The push is async; await a tick.
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(router.currentRoute.value.path).toBe('/families/create')
    })

    it('onChange ignores non-string values', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const { w } = await mountSwitcher()
        const family = useActiveFamilyStore()
        await w.findComponent({ name: 'VSelectStub' }).vm.$emit('update:modelValue', 42)
        expect(family.activeFamilyId).toBeNull()
    })
})
