import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('vuetify', () => ({ useDisplay: () => ({ mobile: { value: false } }) }))

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
                    props: ['modelValue', 'rail'],
                    template:
                        '<div class="drawer-stub" :data-rail="String(rail)" :data-open="String(modelValue)"><slot /><slot name="append" /></div>',
                },
                'v-list': { template: '<div><slot /></div>' },
                'v-list-item': {
                    template: '<a class="li" :data-to="to" :data-icon="prependIcon">{{ title }}</a>',
                    props: ['to', 'title', 'prependIcon', 'activeColor'],
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
})
