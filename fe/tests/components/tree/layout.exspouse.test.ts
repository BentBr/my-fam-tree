// v3.1 layout invariants: ex-spouse adjacency. A person with both an open
// and an ended partnership on the same row is laid out as ONE block:
//   [ended-partner] [shared-person] [open-partner]
// Children of each internal couple sub-cluster under that couple's midpoint
// — Felix (Klaus + Brigitte) sits under the Klaus + Brigitte midpoint,
// Lina/Max under the Klaus + Anna midpoint. The shared `layout.regression.
// test.ts` continues to guard the 1000-person stress + the seeded family.

import { describe, expect, it } from 'vitest'

import { COL_GAP, layoutTree, NODE_W, type TreeInput } from '@/components/tree/layout'

interface PartnerEdgeInput {
    a: string
    b: string
    ended_on?: string | null
}

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

describe('layoutTree (v3.1: ex-spouse adjacency)', () => {
    it('threads ex-partner adjacent to shared person on the LEFT', () => {
        // Klaus is partnered with both Anna (open) and Brigitte (ended).
        // The bug: Brigitte landed at the row's far right; the fix puts
        // her immediately left of Klaus, Anna immediately right.
        const out = layoutTree({
            nodes: [
                person('anna', [], ['klaus']),
                person('brigitte', [], ['klaus']),
                person('klaus', [], ['anna', 'brigitte']),
            ],
            parent_edges: [],
            partner_edges: [
                { a: 'klaus', b: 'anna' },
                { a: 'klaus', b: 'brigitte', ended_on: '2000-06-30' },
            ] as PartnerEdgeInput[],
        })
        const xOf = (id: string): number | undefined => out.nodes.find((n) => n.id === id)?.x
        const yOf = (id: string): number | undefined => out.nodes.find((n) => n.id === id)?.y
        const klausX = xOf('klaus')
        const annaX = xOf('anna')
        const brigitteX = xOf('brigitte')
        expect(klausX).toBeDefined()
        expect(annaX).toBeDefined()
        expect(brigitteX).toBeDefined()
        if (klausX === undefined || annaX === undefined || brigitteX === undefined) return
        // Brigitte (ended) on the left of Klaus, Anna (open) on the right —
        // exactly one card slot apart.
        expect(brigitteX).toBe(klausX - (NODE_W + COL_GAP))
        expect(annaX).toBe(klausX + (NODE_W + COL_GAP))
        // All three on the same row.
        expect(yOf('brigitte')).toBe(yOf('klaus'))
        expect(yOf('anna')).toBe(yOf('klaus'))
    })

    it('centres bio children sub-clusters under their own couple midpoint', () => {
        // One bio child per couple keeps the test free of inter-cluster
        // collision: Felix is Klaus + Brigitte's only bio child, Lina is
        // Klaus + Anna's only bio child. Each must centre under its own
        // couple midpoint.
        const out = layoutTree({
            nodes: [
                person('anna', [], ['klaus']),
                person('brigitte', [], ['klaus']),
                person('klaus', [], ['anna', 'brigitte']),
                person('felix', ['klaus', 'brigitte']),
                person('lina', ['klaus', 'anna']),
            ],
            parent_edges: [
                { a: 'felix', b: 'klaus' },
                { a: 'felix', b: 'brigitte' },
                { a: 'lina', b: 'klaus' },
                { a: 'lina', b: 'anna' },
            ],
            partner_edges: [
                { a: 'klaus', b: 'anna' },
                { a: 'klaus', b: 'brigitte', ended_on: '2000-06-30' },
            ] as PartnerEdgeInput[],
        })
        const xOf = (id: string): number => out.nodes.find((n) => n.id === id)?.x ?? Number.NaN
        const klausX = xOf('klaus')
        const annaX = xOf('anna')
        const brigitteX = xOf('brigitte')
        const felixX = xOf('felix')
        const linaX = xOf('lina')
        // Couple midpoints (using card centres):
        //   Klaus+Brigitte midpoint = (brigitte.centre + klaus.centre) / 2
        const klausCentre = klausX + NODE_W / 2
        const annaCentre = annaX + NODE_W / 2
        const brigitteCentre = brigitteX + NODE_W / 2
        const klausBrigitteMid = (brigitteCentre + klausCentre) / 2
        const klausAnnaMid = (klausCentre + annaCentre) / 2
        const felixCentre = felixX + NODE_W / 2
        const linaCentre = linaX + NODE_W / 2
        // Each only-child centres on its bio couple midpoint within a 1px
        // floating-point tolerance.
        expect(Math.abs(felixCentre - klausBrigitteMid)).toBeLessThanOrEqual(1)
        expect(Math.abs(linaCentre - klausAnnaMid)).toBeLessThanOrEqual(1)
    })

    it('does not regress the plain 2-member couple layout', () => {
        // Sanity: a regular Klaus + Anna couple (no ex-partner) still lays
        // out at Δx = NODE_W + COL_GAP — same as v2/v3.
        const out = layoutTree({
            nodes: [person('anna', [], ['klaus']), person('klaus', [], ['anna'])],
            parent_edges: [],
            partner_edges: [{ a: 'klaus', b: 'anna' }],
        })
        const annaX = out.nodes.find((n) => n.id === 'anna')?.x ?? Number.NaN
        const klausX = out.nodes.find((n) => n.id === 'klaus')?.x ?? Number.NaN
        expect(Math.abs(annaX - klausX)).toBe(NODE_W + COL_GAP)
    })

    it('order: oldest ended partner sits furthest from anchor', () => {
        // Two ended partners on Klaus's row. Oldest divorce year goes to
        // the leftmost slot; more recent ends up adjacent to Klaus.
        const out = layoutTree({
            nodes: [
                person('anna', [], ['klaus']),
                person('brigitte', [], ['klaus']),
                person('clara', [], ['klaus']),
                person('klaus', [], ['anna', 'brigitte', 'clara']),
            ],
            parent_edges: [],
            partner_edges: [
                { a: 'klaus', b: 'anna' },
                { a: 'klaus', b: 'brigitte', ended_on: '2000-06-30' },
                { a: 'klaus', b: 'clara', ended_on: '1985-04-14' },
            ] as PartnerEdgeInput[],
        })
        const xOf = (id: string): number => out.nodes.find((n) => n.id === id)?.x ?? Number.NaN
        // Clara (1985 divorce, oldest) leftmost; Brigitte (2000 divorce)
        // next; then Klaus; Anna (current) rightmost.
        expect(xOf('clara')).toBeLessThan(xOf('brigitte'))
        expect(xOf('brigitte')).toBeLessThan(xOf('klaus'))
        expect(xOf('klaus')).toBeLessThan(xOf('anna'))
    })
})
