import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

// `smAndDown` drives the desktop-rail vs mobile-overlay split; tests flip it to
// assert each branch. Defaults to desktop (false), matching the legacy mock.
const smAndDown = ref(false)
vi.mock('vuetify', () => ({ useDisplay: () => ({ smAndDown }) }))

import NavDrawer from '@/components/layout/NavDrawer.vue'
import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'

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

function mountDrawer() {
    return mount(NavDrawer, {
        global: {
            plugins: [i18n],
            stubs: {
                'v-navigation-drawer': {
                    props: ['modelValue', 'rail', 'temporary', 'permanent'],
                    emits: ['update:modelValue'],
                    template:
                        '<div class="drawer-stub" :data-rail="String(rail)" :data-open="String(modelValue)" :data-temporary="String(temporary)" :data-permanent="String(permanent)" @scrim-close="$emit(\'update:modelValue\', false)"><slot /><slot name="append" /></div>',
                },
                'v-list': { template: '<div><slot /></div>' },
                'v-list-item': {
                    template:
                        '<a class="li" :data-to="to" :data-icon="prependIcon" @click="$emit(\'click\')">{{ title }}</a>',
                    props: ['to', 'title', 'prependIcon', 'activeColor'],
                    emits: ['click'],
                },
                'v-icon': { template: '<i />', props: ['icon', 'size'] },
                RouterLink: {
                    template: '<a class="rl" :data-to="to"><slot /></a>',
                    props: ['to', 'title'],
                },
            },
        },
    })
}

describe('NavDrawer', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        mockStorage()
        smAndDown.value = false
    })

    it('lists the primary nav items; Health is demoted to a footer footnote', () => {
        const w = mountDrawer()
        const items = w.findAll('.li')
        expect(items.map((i) => i.attributes('data-to'))).toEqual(['/tree', '/upcoming'])
        // Health lives in the drawer footer (#append), not the main list.
        const footer = w.find('[data-testid="nav-health-footer"]')
        expect(footer.exists()).toBe(true)
        expect(footer.attributes('data-to')).toBe('/health')
    })

    it('rail mode follows ui.sidebarCollapsed when not mobile', async () => {
        const ui = useUiStore()
        const w = mountDrawer()
        expect(w.find('.drawer-stub').attributes('data-rail')).toBe('false')
        ui.toggleSidebar()
        await w.vm.$nextTick()
        expect(w.find('.drawer-stub').attributes('data-rail')).toBe('true')
    })

    it('on desktop the drawer is permanent and open with no rail by default', () => {
        const w = mountDrawer()
        const drawer = w.find('.drawer-stub')
        expect(drawer.attributes('data-permanent')).toBe('true')
        expect(drawer.attributes('data-temporary')).toBe('false')
        expect(drawer.attributes('data-open')).toBe('true')
        expect(drawer.attributes('data-rail')).toBe('false')
    })

    it('on smAndDown the drawer collapses to a hidden temporary overlay, never rail', () => {
        smAndDown.value = true
        const w = mountDrawer()
        const drawer = w.find('.drawer-stub')
        // Hidden by default (sidebarCollapsed defaults to false) and temporary
        // so it overlays rather than eating the viewport. Never rail on mobile.
        expect(drawer.attributes('data-temporary')).toBe('true')
        expect(drawer.attributes('data-permanent')).toBe('false')
        expect(drawer.attributes('data-open')).toBe('false')
        expect(drawer.attributes('data-rail')).toBe('false')
    })

    it('on smAndDown the hamburger flag opens the overlay (no rail)', async () => {
        smAndDown.value = true
        const ui = useUiStore()
        const w = mountDrawer()
        expect(w.find('.drawer-stub').attributes('data-open')).toBe('false')
        ui.toggleSidebar() // simulate the app-bar hamburger
        await w.vm.$nextTick()
        const drawer = w.find('.drawer-stub')
        expect(drawer.attributes('data-open')).toBe('true')
        expect(drawer.attributes('data-rail')).toBe('false')
    })

    it('on smAndDown dismissing the overlay (scrim/esc) syncs the flag back', async () => {
        smAndDown.value = true
        const ui = useUiStore()
        ui.toggleSidebar() // open it first
        const w = mountDrawer()
        expect(w.find('.drawer-stub').attributes('data-open')).toBe('true')
        await w.find('.drawer-stub').trigger('scrim-close')
        expect(ui.sidebarCollapsed).toBe(false)
        await w.vm.$nextTick()
        expect(w.find('.drawer-stub').attributes('data-open')).toBe('false')
    })

    it('on smAndDown clicking a nav item dismisses the overlay', async () => {
        smAndDown.value = true
        const ui = useUiStore()
        ui.toggleSidebar() // open the overlay
        const w = mountDrawer()
        expect(w.find('.drawer-stub').attributes('data-open')).toBe('true')
        await w.findAll('.li')[0]?.trigger('click')
        expect(ui.sidebarCollapsed).toBe(false)
    })
})
