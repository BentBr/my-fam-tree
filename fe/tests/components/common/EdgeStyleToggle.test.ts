import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import EdgeStyleToggle from '@/components/common/EdgeStyleToggle.vue'
import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'
import { memoryStorage } from '../../helpers/memory-storage'

function mountToggle() {
    return mount(EdgeStyleToggle, {
        global: {
            plugins: [i18n],
            stubs: {
                'v-btn': {
                    template:
                        '<button class="b" :aria-label="$attrs[\'aria-label\']" @click="$emit(\'click\')"><slot /></button>',
                    emits: ['click'],
                },
                'v-icon': { template: '<i :data-icon="$attrs.icon" />' },
            },
        },
    })
}

describe('EdgeStyleToggle', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('localStorage', memoryStorage())
    })

    it('offers the angular style while the default curved lines are active', () => {
        const w = mountToggle()
        expect(w.find('button.b').attributes('aria-label')).toMatch(/angular/i)
        expect(w.find('[data-icon="network"]').exists()).toBe(true)
    })

    it('clicking switches to angular and persists the choice', async () => {
        const ui = useUiStore()
        const w = mountToggle()
        await w.find('button.b').trigger('click')
        expect(ui.edgeStyle).toBe('angular')
        expect(localStorage.getItem('my-fam-tree:edge-style')).toBe('angular')
        // The button now offers the way back.
        expect(w.find('button.b').attributes('aria-label')).toMatch(/curved/i)
        expect(w.find('[data-icon="spline"]').exists()).toBe(true)
    })

    it('a second click returns to curved lines', async () => {
        const ui = useUiStore()
        const w = mountToggle()
        await w.find('button.b').trigger('click')
        await w.find('button.b').trigger('click')
        expect(ui.edgeStyle).toBe('curved')
        expect(localStorage.getItem('my-fam-tree:edge-style')).toBe('curved')
    })
})
