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

    it('applies the highlighted class when isHighlighted is true', () => {
        const w = mount(TreeEdge, {
            props: { kind: 'parent', ax: 0, ay: 0, bx: 100, by: 100, isHighlighted: true },
        })
        // Outer edge-group <g> carries the class. `findAll('g')[0]` is the
        // root because the heart group only exists for partner edges.
        expect(w.find('g').classes()).toContain('highlighted')
    })

    it('applies the dimmed class when isDimmed is true', () => {
        const w = mount(TreeEdge, {
            props: { kind: 'partner', ax: 0, ay: 0, bx: 100, by: 0, isDimmed: true },
        })
        expect(w.find('g').classes()).toContain('dimmed')
    })

    it('omits highlighted/dimmed classes by default', () => {
        const w = mount(TreeEdge, {
            props: { kind: 'parent', ax: 0, ay: 0, bx: 100, by: 100 },
        })
        const cls = w.find('g').classes()
        expect(cls).not.toContain('highlighted')
        expect(cls).not.toContain('dimmed')
    })

    it('renders interlocked rings (not a heart) for a marriage partner edge', () => {
        // Marriage swaps the heart glyph for two adjacent rings. The
        // .rings group + at least two <circle> elements must be in the
        // DOM; the heart group must NOT be.
        const w = mount(TreeEdge, {
            props: {
                kind: 'partner',
                ax: 0,
                ay: 0,
                bx: 200,
                by: 0,
                partnershipKind: 'marriage',
            },
        })
        const ringsGroup = w.findAll('g').find((g) => g.classes('rings'))
        expect(ringsGroup, 'marriage edge has a .rings <g>').toBeDefined()
        expect(ringsGroup?.findAll('circle').length).toBe(2)
        expect(w.findAll('g').find((g) => g.classes('heart'))).toBeUndefined()
        // Outer group carries the .marriage class so the stylesheet can
        // pick the gold token; .ended is absent on an active marriage.
        const cls = w.find('g').classes()
        expect(cls).toContain('marriage')
        expect(cls).not.toContain('ended')
    })

    it('keeps the heart for non-marriage partnerships (civil_union, partnership, null)', () => {
        // The glyph only changes for `kind === 'marriage'`; civil unions
        // and registered partnerships continue to read as rose hearts.
        for (const kind of ['civil_union', 'partnership', null] as const) {
            const w = mount(TreeEdge, {
                props: {
                    kind: 'partner',
                    ax: 0,
                    ay: 0,
                    bx: 200,
                    by: 0,
                    partnershipKind: kind,
                },
            })
            const heartGroup = w.findAll('g').find((g) => g.classes('heart'))
            expect(heartGroup, `${String(kind)} keeps the heart`).toBeDefined()
            expect(w.findAll('g').find((g) => g.classes('rings'))).toBeUndefined()
            expect(w.find('g').classes()).not.toContain('marriage')
        }
    })

    it('suppresses the connector line for directly-adjacent partner pairs', () => {
        // When the two partners are visually adjacent on the canvas, the
        // midpoint glyph (heart or rings) is the connector — the dashed
        // line behind it is redundant. The renderer drops it: the .edge
        // <path> must not appear when directlyAdjacent === true. The
        // glyph still renders.
        const w = mount(TreeEdge, {
            props: {
                kind: 'partner',
                ax: 0,
                ay: 0,
                bx: 200,
                by: 0,
                partnershipKind: 'marriage',
                directlyAdjacent: true,
            },
        })
        expect(w.find('path.edge').exists()).toBe(false)
        // Rings glyph still present.
        expect(w.findAll('g').find((g) => g.classes('rings'))).toBeDefined()
    })

    it('keeps the connector line for non-adjacent (long-span) partner pairs', () => {
        // The default case (directlyAdjacent absent / false) is the
        // "long" partnership routed past an intermediate row member —
        // the midpoint glyph hides behind the intermediate node and
        // the line is the ONLY visual cue. Must render.
        const w = mount(TreeEdge, {
            props: {
                kind: 'partner',
                ax: 0,
                ay: 0,
                bx: 200,
                by: 0,
                partnershipKind: 'partnership',
                directlyAdjacent: false,
            },
        })
        expect(w.find('path.edge').exists()).toBe(true)
    })

    it('flags the edge as ended when the partnership ended_on is set', () => {
        // The `ended` flag drives the CSS class only — the actual grey
        // colour is applied by the scoped stylesheet. We pin the class
        // shape here so a future refactor that drops the modifier
        // (and loses the styling) surfaces in unit tests.
        const w = mount(TreeEdge, {
            props: {
                kind: 'partner',
                ax: 0,
                ay: 0,
                bx: 200,
                by: 0,
                partnershipKind: 'marriage',
                ended: true,
            },
        })
        const cls = w.find('g').classes()
        expect(cls).toContain('ended')
        expect(cls).toContain('marriage')
        // The glyph is still the rings (an ended marriage is still a
        // marriage); only the colour shifts via CSS.
        expect(w.findAll('g').find((g) => g.classes('rings'))).toBeDefined()
    })
})
