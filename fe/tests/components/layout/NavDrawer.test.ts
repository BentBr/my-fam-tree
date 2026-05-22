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
                        '<div class="drawer-stub" :data-rail="String(rail)" :data-open="String(modelValue)"><slot /></div>',
                },
                'v-list': { template: '<div><slot /></div>' },
                'v-list-item': {
                    template: '<a class="li" :data-to="to" :data-icon="prependIcon">{{ title }}</a>',
                    props: ['to', 'title', 'prependIcon', 'activeColor'],
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

    it('renders the live nav items only (no /reminders/* yet)', () => {
        // /reminders/history is wired by Phase 4b. Until the route exists,
        // the nav must not list it — otherwise vue-router emits a
        // "No match found" warn on every navigation.
        const w = mountDrawer()
        const items = w.findAll('.li')
        expect(items).toHaveLength(3)
        expect(items.map((i) => i.attributes('data-to'))).toEqual(['/tree', '/upcoming', '/health'])
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
