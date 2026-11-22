import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import { i18n } from '@/i18n'
import TreeModeToggles from '@/views/tree/TreeModeToggles.vue'

const VBtn = {
    props: ['icon', 'prependIcon', 'active', 'variant'],
    emits: ['click'],
    template:
        '<button type="button" :data-icon-only="icon !== undefined" :data-active="active" @click="$emit(\'click\')"><slot /></button>',
}

function mountToggles(props: { compact: boolean; highlight: boolean; compare: boolean }) {
    return mount(TreeModeToggles, { props, global: { plugins: [i18n], stubs: { 'v-btn': VBtn } } })
}

describe('TreeModeToggles', () => {
    it('renders one labelled toggle per mode on wide toolbars', () => {
        const w = mountToggles({ compact: false, highlight: false, compare: true })
        const highlight = w.get('[data-testid="tree-highlight-mode"]')
        const compare = w.get('[data-testid="tree-compare-mode"]')
        expect(w.findAll('button')).toHaveLength(2)
        expect(highlight.text()).toBe('Highlight mode')
        expect(compare.text()).toBe('Compare')
        expect(highlight.attributes('data-active')).toBe('false')
        expect(compare.attributes('data-active')).toBe('true')
    })

    it('renders icon-only toggles with an accessible name when compact', () => {
        const w = mountToggles({ compact: true, highlight: true, compare: false })
        const highlight = w.get('[data-testid="tree-highlight-mode"]')
        const compare = w.get('[data-testid="tree-compare-mode"]')
        expect(w.findAll('button')).toHaveLength(2)
        expect(highlight.attributes('data-icon-only')).toBe('true')
        expect(highlight.text()).toBe('')
        expect(highlight.attributes('aria-label')).toBe('Highlight mode')
        expect(compare.attributes('aria-label')).toBe('Compare')
        expect(highlight.attributes('data-active')).toBe('true')
    })

    it('reports which toggle was clicked', async () => {
        const w = mountToggles({ compact: false, highlight: false, compare: false })
        await w.get('[data-testid="tree-highlight-mode"]').trigger('click')
        await w.get('[data-testid="tree-compare-mode"]').trigger('click')
        expect(w.emitted('toggle-highlight')).toHaveLength(1)
        expect(w.emitted('toggle-compare')).toHaveLength(1)
    })
})
