import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import TreeEdge from '@/components/tree/TreeEdge.vue'

describe('TreeEdge', () => {
    it('renders a cubic Bezier path for parent edges', () => {
        const w = mount(TreeEdge, {
            props: { kind: 'parent', ax: 0, ay: 0, bx: 100, by: 200 },
        })
        const d = w.find('path').attributes('d')
        expect(d).toMatch(/^M 0 0 C/)
        expect(d).toContain('100 200')
    })

    it('renders a straight line for partner edges and a heart glyph', () => {
        const w = mount(TreeEdge, {
            props: { kind: 'partner', ax: 0, ay: 0, bx: 200, by: 0 },
        })
        const html = w.html()
        expect(html).toContain('M 0 0 L 200 0')
        // Two paths: the edge + the heart shape.
        expect(w.findAll('path').length).toBeGreaterThanOrEqual(2)
    })

    it('puts the heart at the midpoint via a translate', () => {
        const w = mount(TreeEdge, {
            props: { kind: 'partner', ax: 0, ay: 0, bx: 200, by: 100 },
        })
        const heartG = w.findAll('g').find((g) => g.classes('heart'))
        expect(heartG).toBeDefined()
        // midX = 100, midY = 50; translated by midX-6, midY-6 → 94, 44.
        expect(heartG?.attributes('transform')).toContain('translate(94')
    })
})
