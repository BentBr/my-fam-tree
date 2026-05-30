import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import ThemeToggle from '@/components/common/ThemeToggle.vue'
import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'

// Stub the prefers-color-scheme query so `currentResolvedTheme()`
// returns a deterministic value when the store's mode is `system`.
function stubMatchMedia(prefersDark: boolean): void {
    vi.stubGlobal('matchMedia', (q: string) => ({
        matches: q.includes('dark') ? prefersDark : !prefersDark,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
    }))
}

function stubStorage(): void {
    const store: Record<string, string> = {}
    vi.stubGlobal('localStorage', {
        getItem: (k: string) => store[k] ?? null,
        setItem: (k: string, v: string) => {
            store[k] = v
        },
        removeItem: (k: string) => {
            delete store[k]
        },
        clear: () => {
            for (const k of Object.keys(store)) delete store[k]
        },
        key: (i: number) => Object.keys(store)[i] ?? null,
        get length() {
            return Object.keys(store).length
        },
    })
}

describe('ThemeToggle', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        stubStorage()
        stubMatchMedia(false)
    })

    it('renders a single icon button labelled with the next-state hint', () => {
        const w = mount(ThemeToggle, {
            global: {
                plugins: [i18n],
                stubs: {
                    'v-btn': { template: '<button class="b" :aria-label="$attrs[\'aria-label\']"><slot /></button>' },
                    'v-icon': { template: '<i :data-icon="$attrs.icon" />' },
                },
            },
        })
        const btn = w.find('button.b')
        expect(btn.exists()).toBe(true)
        // System resolves to light (matchMedia stubbed false) → toggle to dark.
        expect(btn.attributes('aria-label')).toMatch(/dark/i)
        expect(w.find('[data-icon="moon"]').exists()).toBe(true)
    })

    it('toggling persists an explicit theme choice', async () => {
        const ui = useUiStore()
        const w = mount(ThemeToggle, {
            global: {
                plugins: [i18n],
                stubs: {
                    'v-btn': {
                        template: '<button class="b" @click="$emit(\'click\')"><slot /></button>',
                        emits: ['click'],
                    },
                    'v-icon': true,
                },
            },
        })
        await w.find('button.b').trigger('click')
        expect(ui.theme).toBe('dark')
    })
})
