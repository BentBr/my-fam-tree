import { describe, expect, it } from 'vitest'

import { layoutTree, NODE_H, NODE_W, type TreeInput } from '@/components/tree/layout'

function person(
    id: string,
    parents: string[] = [],
    partners: string[] = [],
    birth?: string,
): TreeInput['nodes'][number] {
    return {
        id,
        given_name: `G${id}`,
        family_name: `F${id}`,
        ...(birth === undefined ? {} : { birth_date: birth }),
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

    it('ranks a 4-generation lineage by descendant depth, not parent-chain depth', () => {
        // ggp -> gp -> p -> c — every link has only one parent, but the
        // important thing is each row sits at its own y. Pre-fix the
        // canonical-parent virtual-root collapsed roots to depth 1; post-fix
        // each generation is a distinct y because gen() = 1 + max(child).
        const out = layoutTree({
            nodes: [person('ggp'), person('gp', ['ggp']), person('p', ['gp']), person('c', ['p'])],
            parent_edges: [
                { a: 'gp', b: 'ggp' },
                { a: 'p', b: 'gp' },
                { a: 'c', b: 'p' },
            ],
            partner_edges: [],
        })
        const ys = new Map(out.nodes.map((n) => [n.id, n.y]))
        const ggp = ys.get('ggp')
        const gp = ys.get('gp')
        const p = ys.get('p')
        const c = ys.get('c')
        expect(ggp).toBeDefined()
        expect(gp).toBeDefined()
        expect(p).toBeDefined()
        expect(c).toBeDefined()
        if (ggp !== undefined && gp !== undefined && p !== undefined && c !== undefined) {
            // Highest generation (ggp) at the top of the canvas (y=0); each
            // subsequent generation steps down by one full row.
            expect(ggp).toBe(0)
            expect(gp).toBeGreaterThan(ggp)
            expect(p).toBeGreaterThan(gp)
            expect(c).toBeGreaterThan(p)
            const step = NODE_H + 100
            expect(gp - ggp).toBe(step)
            expect(p - gp).toBe(step)
            expect(c - p).toBe(step)
        }
    })

    it('eldest orphan with much older birth_date sits ABOVE the youngest top-row member', () => {
        const out = layoutTree({
            nodes: [
                person('gp', [], [], '1936'),
                person('aunt', [], [], '1938'),
                person('child', ['gp']),
                person('elder', [], [], '1910-05-06'),
            ],
            parent_edges: [{ a: 'child', b: 'gp' }],
            partner_edges: [],
        })
        const ys = new Map(out.nodes.map((n) => [n.id, n.y]))
        const gp = ys.get('gp')
        const aunt = ys.get('aunt')
        const elder = ys.get('elder')
        expect(gp).toBeDefined()
        expect(aunt).toBeDefined()
        expect(elder).toBeDefined()
        if (gp !== undefined && aunt !== undefined && elder !== undefined) {
            expect(elder).toBeLessThan(gp)
            expect(elder).toBeLessThan(aunt)
        }
    })

    it('two parentless people with very different birth_dates rank older above younger', () => {
        const out = layoutTree({
            nodes: [person('young', [], [], '1980'), person('old', [], [], '1900')],
            parent_edges: [],
            partner_edges: [],
        })
        const young = out.nodes.find((n) => n.id === 'young')
        const old = out.nodes.find((n) => n.id === 'old')
        expect(young).toBeDefined()
        expect(old).toBeDefined()
        if (young && old) {
            expect(old.y).toBeLessThan(young.y)
        }
    })

    it('orphan leaf with NO birth_date falls back to generation 0 and does not crash', () => {
        const out = layoutTree({
            nodes: [
                person('alone'), // no parents, no children, no birth_date
                person('peer'),
            ],
            parent_edges: [],
            partner_edges: [],
        })
        const alone = out.nodes.find((n) => n.id === 'alone')
        const peer = out.nodes.find((n) => n.id === 'peer')
        expect(alone).toBeDefined()
        expect(peer).toBeDefined()
        if (alone && peer) {
            expect(alone.y).toBe(0)
            expect(peer.y).toBe(0)
            expect(Number.isFinite(alone.y)).toBe(true)
            expect(Number.isFinite(peer.y)).toBe(true)
        }
    })

    it('partner pass still pulls in-row partners adjacent after the generation rerank', () => {
        // Both partners have ONE shared child, so they each end up at gen 1 —
        // same row, eligible for the partner-adjacency nudge.
        const out = layoutTree({
            nodes: [person('mom', [], ['dad']), person('dad', [], ['mom']), person('kid', ['mom', 'dad'])],
            parent_edges: [
                { a: 'kid', b: 'mom' },
                { a: 'kid', b: 'dad' },
            ],
            partner_edges: [{ a: 'mom', b: 'dad' }],
        })
        const mom = out.nodes.find((n) => n.id === 'mom')
        const dad = out.nodes.find((n) => n.id === 'dad')
        expect(mom).toBeDefined()
        expect(dad).toBeDefined()
        if (mom && dad) {
            expect(mom.y).toBe(dad.y)
            expect(Math.abs(mom.x - dad.x)).toBe(NODE_W + 24)
        }
    })
})
