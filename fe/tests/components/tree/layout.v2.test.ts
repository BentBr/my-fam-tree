// v2 layout invariants: couples adjacent, sibling clusters separated by
// CLUSTER_GAP, in-cluster spacing at COL_GAP, children sorted by birth.
// Split out of layout.test.ts to keep each file under the 500-line cap.
import { describe, expect, it } from 'vitest'

import { layoutTree, NODE_W } from '@/components/tree/layout'

const COL_GAP = 24
const CLUSTER_GAP = COL_GAP * 2

describe('layoutTree (v2: couples + clusters)', () => {
    it("centers a couple's children cluster on the couple midpoint", () => {
        const out = layoutTree({
            nodes: [
                { id: 'dad', given_name: 'Dad', family_name: 'X', parent_ids: [], partner_ids: ['mom'] },
                { id: 'mom', given_name: 'Mom', family_name: 'X', parent_ids: [], partner_ids: ['dad'] },
                {
                    id: 'k1',
                    given_name: 'K1',
                    family_name: 'X',
                    birth_date: '2000',
                    parent_ids: ['dad', 'mom'],
                    partner_ids: [],
                },
                {
                    id: 'k2',
                    given_name: 'K2',
                    family_name: 'X',
                    birth_date: '2002',
                    parent_ids: ['dad', 'mom'],
                    partner_ids: [],
                },
                {
                    id: 'k3',
                    given_name: 'K3',
                    family_name: 'X',
                    birth_date: '2004',
                    parent_ids: ['dad', 'mom'],
                    partner_ids: [],
                },
            ],
            parent_edges: [
                { a: 'k1', b: 'dad' },
                { a: 'k1', b: 'mom' },
                { a: 'k2', b: 'dad' },
                { a: 'k2', b: 'mom' },
                { a: 'k3', b: 'dad' },
                { a: 'k3', b: 'mom' },
            ],
            partner_edges: [{ a: 'dad', b: 'mom' }],
        })
        const dad = out.nodes.find((n) => n.id === 'dad')
        const mom = out.nodes.find((n) => n.id === 'mom')
        const k1 = out.nodes.find((n) => n.id === 'k1')
        const k3 = out.nodes.find((n) => n.id === 'k3')
        expect(dad).toBeDefined()
        expect(mom).toBeDefined()
        expect(k1).toBeDefined()
        expect(k3).toBeDefined()
        if (dad && mom && k1 && k3) {
            expect(dad.y).toBe(mom.y)
            expect(Math.abs(dad.x - mom.x)).toBe(NODE_W + COL_GAP)
            const coupleMid = (Math.min(dad.x, mom.x) + Math.max(dad.x, mom.x) + NODE_W) / 2
            const childMid = (k1.x + 0 + (k3.x + NODE_W)) / 2
            expect(Math.abs(childMid - coupleMid)).toBeLessThanOrEqual(1)
        }
    })

    // Repro of the user-reported "Werner – Hannelore – Greta – Otto" visual
    // bug. Each couple stays adjacent (Δx == NODE_W + COL_GAP) on its row.
    it('keeps partners adjacent in a two-couple/two-grandchild layout', () => {
        const out = layoutTree({
            nodes: [
                {
                    id: 'otto',
                    given_name: 'O',
                    family_name: 'M',
                    birth_date: '1935',
                    parent_ids: [],
                    partner_ids: ['hannelore'],
                },
                {
                    id: 'hannelore',
                    given_name: 'H',
                    family_name: 'M',
                    birth_date: '1938',
                    parent_ids: [],
                    partner_ids: ['otto'],
                },
                {
                    id: 'werner',
                    given_name: 'W',
                    family_name: 'S',
                    birth_date: '1936',
                    parent_ids: [],
                    partner_ids: ['greta'],
                },
                {
                    id: 'greta',
                    given_name: 'G',
                    family_name: 'S',
                    birth_date: '1940',
                    parent_ids: [],
                    partner_ids: ['werner'],
                },
                {
                    id: 'klaus',
                    given_name: 'K',
                    family_name: 'M',
                    birth_date: '1965',
                    parent_ids: ['otto', 'hannelore'],
                    partner_ids: ['anna'],
                },
                {
                    id: 'anna',
                    given_name: 'A',
                    family_name: 'M',
                    birth_date: '1968',
                    parent_ids: ['werner', 'greta'],
                    partner_ids: ['klaus'],
                },
            ],
            parent_edges: [
                { a: 'klaus', b: 'otto' },
                { a: 'klaus', b: 'hannelore' },
                { a: 'anna', b: 'werner' },
                { a: 'anna', b: 'greta' },
            ],
            partner_edges: [
                { a: 'otto', b: 'hannelore' },
                { a: 'werner', b: 'greta' },
                { a: 'klaus', b: 'anna' },
            ],
        })
        const dx = (a: string, b: string): number => {
            const na = out.nodes.find((n) => n.id === a)
            const nb = out.nodes.find((n) => n.id === b)
            if (na === undefined || nb === undefined) throw new Error('missing node')
            return Math.abs(na.x - nb.x)
        }
        expect(dx('otto', 'hannelore')).toBe(NODE_W + COL_GAP)
        expect(dx('werner', 'greta')).toBe(NODE_W + COL_GAP)
        expect(dx('klaus', 'anna')).toBe(NODE_W + COL_GAP)
        const yOf = (id: string): number => out.nodes.find((n) => n.id === id)?.y ?? -1
        expect(yOf('otto')).toBe(yOf('hannelore'))
        expect(yOf('werner')).toBe(yOf('greta'))
    })

    it('separates two sibling clusters by at least CLUSTER_GAP', () => {
        // Two couples on row 0, each with one child on row 1. The two row-1
        // singletons are NOT siblings — they belong to different parent
        // blocks — so they must be separated by ≥ CLUSTER_GAP.
        const out = layoutTree({
            nodes: [
                { id: 'a1', given_name: 'A1', family_name: 'X', parent_ids: [], partner_ids: ['a2'] },
                { id: 'a2', given_name: 'A2', family_name: 'X', parent_ids: [], partner_ids: ['a1'] },
                { id: 'b1', given_name: 'B1', family_name: 'Y', parent_ids: [], partner_ids: ['b2'] },
                { id: 'b2', given_name: 'B2', family_name: 'Y', parent_ids: [], partner_ids: ['b1'] },
                { id: 'ca', given_name: 'Ca', family_name: 'X', parent_ids: ['a1', 'a2'], partner_ids: [] },
                { id: 'cb', given_name: 'Cb', family_name: 'Y', parent_ids: ['b1', 'b2'], partner_ids: [] },
            ],
            parent_edges: [
                { a: 'ca', b: 'a1' },
                { a: 'ca', b: 'a2' },
                { a: 'cb', b: 'b1' },
                { a: 'cb', b: 'b2' },
            ],
            partner_edges: [
                { a: 'a1', b: 'a2' },
                { a: 'b1', b: 'b2' },
            ],
        })
        const ca = out.nodes.find((n) => n.id === 'ca')
        const cb = out.nodes.find((n) => n.id === 'cb')
        expect(ca).toBeDefined()
        expect(cb).toBeDefined()
        if (ca && cb) {
            const left = ca.x < cb.x ? ca : cb
            const right = ca.x < cb.x ? cb : ca
            const gap = right.x - (left.x + NODE_W)
            expect(gap).toBeGreaterThanOrEqual(CLUSTER_GAP)
        }
    })

    it('uses COL_GAP between siblings inside one cluster', () => {
        // Same parents → siblings → tight COL_GAP, not CLUSTER_GAP.
        const out = layoutTree({
            nodes: [
                { id: 'p1', given_name: 'P1', family_name: 'X', parent_ids: [], partner_ids: ['p2'] },
                { id: 'p2', given_name: 'P2', family_name: 'X', parent_ids: [], partner_ids: ['p1'] },
                {
                    id: 'k1',
                    given_name: 'K1',
                    family_name: 'X',
                    birth_date: '2000',
                    parent_ids: ['p1', 'p2'],
                    partner_ids: [],
                },
                {
                    id: 'k2',
                    given_name: 'K2',
                    family_name: 'X',
                    birth_date: '2002',
                    parent_ids: ['p1', 'p2'],
                    partner_ids: [],
                },
            ],
            parent_edges: [
                { a: 'k1', b: 'p1' },
                { a: 'k1', b: 'p2' },
                { a: 'k2', b: 'p1' },
                { a: 'k2', b: 'p2' },
            ],
            partner_edges: [{ a: 'p1', b: 'p2' }],
        })
        const k1 = out.nodes.find((n) => n.id === 'k1')
        const k2 = out.nodes.find((n) => n.id === 'k2')
        expect(k1).toBeDefined()
        expect(k2).toBeDefined()
        if (k1 && k2) {
            const left = k1.x < k2.x ? k1 : k2
            const right = k1.x < k2.x ? k2 : k1
            const gap = right.x - (left.x + NODE_W)
            expect(gap).toBe(COL_GAP)
        }
    })

    it('sorts siblings left-to-right by birth date (oldest first)', () => {
        // Three siblings; oldest-to-youngest must place left-to-right.
        const out = layoutTree({
            nodes: [
                { id: 'p1', given_name: 'P1', family_name: 'X', parent_ids: [], partner_ids: ['p2'] },
                { id: 'p2', given_name: 'P2', family_name: 'X', parent_ids: [], partner_ids: ['p1'] },
                {
                    id: 'youngest',
                    given_name: 'Y',
                    family_name: 'X',
                    birth_date: '2005',
                    parent_ids: ['p1', 'p2'],
                    partner_ids: [],
                },
                {
                    id: 'middle',
                    given_name: 'M',
                    family_name: 'X',
                    birth_date: '2002',
                    parent_ids: ['p1', 'p2'],
                    partner_ids: [],
                },
                {
                    id: 'oldest',
                    given_name: 'O',
                    family_name: 'X',
                    birth_date: '1998',
                    parent_ids: ['p1', 'p2'],
                    partner_ids: [],
                },
            ],
            parent_edges: [
                { a: 'youngest', b: 'p1' },
                { a: 'youngest', b: 'p2' },
                { a: 'middle', b: 'p1' },
                { a: 'middle', b: 'p2' },
                { a: 'oldest', b: 'p1' },
                { a: 'oldest', b: 'p2' },
            ],
            partner_edges: [{ a: 'p1', b: 'p2' }],
        })
        const o = out.nodes.find((n) => n.id === 'oldest')
        const m = out.nodes.find((n) => n.id === 'middle')
        const y = out.nodes.find((n) => n.id === 'youngest')
        expect(o).toBeDefined()
        expect(m).toBeDefined()
        expect(y).toBeDefined()
        if (o && m && y) {
            expect(o.x).toBeLessThan(m.x)
            expect(m.x).toBeLessThan(y.x)
        }
    })
})
