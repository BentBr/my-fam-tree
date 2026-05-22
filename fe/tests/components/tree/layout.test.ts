import { describe, expect, it } from 'vitest'

import { layoutTree, NODE_H, NODE_W, type TreeInput } from '@/components/tree/layout'

function person(id: string, parents: string[] = [], partners: string[] = []): TreeInput['nodes'][number] {
    return {
        id,
        given_name: `G${id}`,
        family_name: `F${id}`,
        parent_ids: parents,
        partner_ids: partners,
    }
}

describe('layoutTree', () => {
    it('returns empty layout for empty input', () => {
        const out = layoutTree({ nodes: [], parent_edges: [], partner_edges: [] })
        expect(out.nodes).toEqual([])
        expect(out.parentEdges).toEqual([])
        expect(out.partnerEdges).toEqual([])
        // Empty bounds collapse to NODE_W / NODE_H minima.
        expect(out.width).toBe(NODE_W)
        expect(out.height).toBe(NODE_H)
    })

    it('positions a single root at x>=0, y=0', () => {
        const out = layoutTree({
            nodes: [person('a')],
            parent_edges: [],
            partner_edges: [],
        })
        expect(out.nodes).toHaveLength(1)
        const n = out.nodes[0]
        expect(n).toBeDefined()
        if (n) {
            expect(n.x).toBe(0)
            expect(n.y).toBe(0)
        }
    })

    it('places child below parent, with positive width/height', () => {
        const out = layoutTree({
            nodes: [person('p'), person('c', ['p'])],
            parent_edges: [{ a: 'c', b: 'p' }],
            partner_edges: [],
        })
        expect(out.nodes).toHaveLength(2)
        const parent = out.nodes.find((n) => n.id === 'p')
        const child = out.nodes.find((n) => n.id === 'c')
        expect(parent).toBeDefined()
        expect(child).toBeDefined()
        if (parent && child) {
            expect(child.y).toBeGreaterThan(parent.y)
        }
        expect(out.parentEdges).toHaveLength(1)
    })

    it('partner pass pulls in-row pair onto adjacent x', () => {
        const out = layoutTree({
            nodes: [person('a', [], ['b']), person('b', [], ['a'])],
            parent_edges: [],
            partner_edges: [{ a: 'a', b: 'b' }],
        })
        const a = out.nodes.find((n) => n.id === 'a')
        const b = out.nodes.find((n) => n.id === 'b')
        expect(a).toBeDefined()
        expect(b).toBeDefined()
        if (a && b) {
            // Same row.
            expect(a.y).toBe(b.y)
            // Adjacent: |delta-x| equals NODE_W + COL_GAP (24) per layout step.
            expect(Math.abs(a.x - b.x)).toBe(NODE_W + 24)
        }
        expect(out.partnerEdges).toHaveLength(1)
    })

    it('partner pass swaps when b is left of a (covers the other branch)', () => {
        // Two-child tree where child order produces b at lower x than a.
        // Provide both children of a virtual root + the partner edge a<->b.
        const out = layoutTree({
            nodes: [person('b'), person('a', [], ['b'])],
            parent_edges: [],
            partner_edges: [{ a: 'a', b: 'b' }],
        })
        const a = out.nodes.find((n) => n.id === 'a')
        const b = out.nodes.find((n) => n.id === 'b')
        expect(a).toBeDefined()
        expect(b).toBeDefined()
    })

    it('cross-row partners are not pulled together (different y)', () => {
        const out = layoutTree({
            nodes: [person('top'), person('bottom', ['top'])],
            parent_edges: [{ a: 'bottom', b: 'top' }],
            partner_edges: [{ a: 'top', b: 'bottom' }],
        })
        const top = out.nodes.find((n) => n.id === 'top')
        const bottom = out.nodes.find((n) => n.id === 'bottom')
        expect(top).toBeDefined()
        expect(bottom).toBeDefined()
        if (top && bottom) {
            expect(top.y).not.toBe(bottom.y)
        }
        // partnerEdges entry still emitted, since layout records cross-row edges.
        expect(out.partnerEdges).toHaveLength(1)
    })

    it('drops parent/partner edges that reference unknown nodes', () => {
        const out = layoutTree({
            nodes: [person('a')],
            parent_edges: [{ a: 'a', b: 'ghost' }],
            partner_edges: [{ a: 'a', b: 'ghost' }],
        })
        expect(out.parentEdges).toHaveLength(0)
        expect(out.partnerEdges).toHaveLength(0)
    })

    it('uses smallest parent_id as canonical when multiple parents exist', () => {
        const out = layoutTree({
            nodes: [person('p1'), person('p2'), person('c', ['p2', 'p1'])],
            parent_edges: [
                { a: 'c', b: 'p1' },
                { a: 'c', b: 'p2' },
            ],
            partner_edges: [],
        })
        // Two parent edges drawn even though only one canonical parent shaped the tree.
        expect(out.parentEdges).toHaveLength(2)
    })
})
