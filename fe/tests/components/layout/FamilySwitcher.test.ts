import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))
// Mock Vuetify's `useDisplay` so the component sees a deterministic
// breakpoint. Default is desktop (smAndDown = false); tests that
// exercise the icon-only mobile variant flip it to true.
const smAndDown = ref(false)
vi.mock('vuetify', () => ({ useDisplay: () => ({ smAndDown }) }))

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
    // FamilySwitcher calls `useQueryClient()` to invalidate caches on family
    // switch; mount needs a QueryClient in scope. `retry: 0` keeps the
    // post-change invalidate a no-op in this stub context.
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: 0 } } })
    const w = mount(FamilySwitcher, {
        global: {
            plugins: [i18n, router, [VueQueryPlugin, { queryClient }]],
            stubs: {
                VSelect: {
                    name: 'VSelectStub',
                    props: ['modelValue', 'items'],
                    emits: ['update:modelValue'],
                    template: '<div class="select-stub" :data-items="JSON.stringify(items)" />',
                },
                // Mobile-variant stubs. The v-menu's activator slot is exposed
                // so the test can resolve the icon button via `data-testid`,
                // and the list items render as clickable divs so the click
                // path lands in `onChange` exactly like the v-select emit.
                VMenu: {
                    name: 'VMenuStub',
                    template: '<div class="menu-stub"><slot name="activator" :props="{}" /><slot /></div>',
                },
                VBtn: {
                    name: 'VBtnStub',
                    props: ['icon', 'variant', 'density'],
                    template:
                        '<button type="button" :data-testid="$attrs[\'data-testid\']" :data-icon="icon" @click="$emit(\'click\', $event)"><slot /></button>',
                    emits: ['click'],
                },
                VList: { template: '<ul class="list-stub" :data-testid="$attrs[\'data-testid\']"><slot /></ul>' },
                VListItem: {
                    name: 'VListItemStub',
                    props: ['active', 'title', 'subtitle', 'prependIcon'],
                    template:
                        '<li class="list-item-stub" :data-active="active" :data-title="title" @click="$emit(\'click\', $event)"><slot /></li>',
                    emits: ['click'],
                },
                VDivider: { template: '<li class="divider-stub" />' },
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
        // Reset to desktop default between tests; the mobile-variant
        // test flips it explicitly.
        smAndDown.value = false
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

    it('collapses to an icon-only menu activator on smAndDown', async () => {
        // On mobile the wide v-select is replaced by an icon-only button
        // ("users" glyph) that opens a v-menu with the same items list.
        // Pin the activator presence + icon + that the v-select is NOT
        // rendered so a future regression that drops one variant surfaces.
        smAndDown.value = true
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const { w } = await mountSwitcher()
        // The icon button replaces the v-select entirely.
        expect(w.find('.select-stub').exists()).toBe(false)
        const activator = w.find('[data-testid="family-switcher"]')
        expect(activator.exists()).toBe(true)
        expect(activator.attributes('data-icon')).toBe('users')
    })

    it('on smAndDown, clicking a family list-item switches the active family', async () => {
        // Same selection contract as the desktop v-select emit, but via
        // a list-item click inside the v-menu.
        smAndDown.value = true
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const { w } = await mountSwitcher()
        const family = useActiveFamilyStore()
        // First list-item is the F1 family; click it.
        const items = w.findAll('.list-item-stub')
        expect(items.length).toBeGreaterThan(0)
        await items[0]?.trigger('click')
        expect(family.activeFamilyId).toBe('f-1')
    })
})
