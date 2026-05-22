import { describe, expect, it } from 'vitest'

import { layoutTree, NODE_H, NODE_W, type TreeInput } from '@/components/tree/layout'

const COL_GAP = 24
const CLUSTER_GAP = COL_GAP * 2

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

// Repro for a regression filed visually: after adding "peter old" (1910) as
// Otto's parent, Hannelore and Greta vanish from the canvas. The API
// payload still has all 9 persons; the bug is purely in layoutTree.
describe('layoutTree (regression: grandparent does not drop nodes)', () => {
    it('returns all 9 persons in the seeded family + added grandparent', () => {
        // Use a UUID that sorts AFTER the seeded ids — this mirrors how
        // `crypto.randomUUID()` lands in the backend's `persons` list query
        // result (default ORDER BY id). The seeded ids start with
        // `00000003-...`, the FE-generated ones start with random hex.
        // The API list order then dictates the loop order in `layoutTree`,
        // which dictates root.children order, which dictates d3-tree x.
        const peter = 'c836a69f-bd6e-42df-abc6-50ceab0f815d'
        const otto = '00000003-0000-0000-0000-000000000001'
        const hannelore = '00000003-0000-0000-0000-000000000002'
        const werner = '00000003-0000-0000-0000-000000000003'
        const greta = '00000003-0000-0000-0000-000000000004'
        const klaus = '00000003-0000-0000-0000-000000000005'
        const anna = '00000003-0000-0000-0000-000000000006'
        const lina = '00000003-0000-0000-0000-000000000007'
        const max = '00000003-0000-0000-0000-000000000008'
        // Match the API's default ORDER BY id ordering: seeded persons
        // (00000003-...) come first, then any FE-added ones. The order
        // of input.nodes drives the iteration order of canonical-children
        // map insertion, which drives the d3-tree sibling layout.
        const out = layoutTree({
            nodes: [
                person(otto, [peter], [hannelore], '1935-03-12'),
                person(hannelore, [], [otto], '1938-07-23'),
                person(werner, [], [greta], '1936-05-18'),
                person(greta, [], [werner], '1940-02-09'),
                person(klaus, [otto, hannelore], [anna], '1965-04-22'),
                person(anna, [werner, greta], [klaus], '1968-08-11'),
                person(lina, [klaus, anna], [], '1995-12-03'),
                person(max, [klaus, anna], [], '1998-04-17'),
                person(peter, [], [], '1910-05-20'),
            ],
            parent_edges: [
                { a: otto, b: peter },
                { a: klaus, b: otto },
                { a: klaus, b: hannelore },
                { a: anna, b: werner },
                { a: anna, b: greta },
                { a: lina, b: klaus },
                { a: lina, b: anna },
                { a: max, b: klaus },
                { a: max, b: anna },
            ],
            partner_edges: [
                { a: otto, b: hannelore },
                { a: werner, b: greta },
                { a: klaus, b: anna },
            ],
        })
        const ids = out.nodes.map((n) => n.id).sort()
        expect(ids).toEqual([peter, otto, hannelore, werner, greta, klaus, anna, lina, max].sort())
        // Sanity: Hannelore and Greta land on the same generation row as
        // their partners Otto and Werner (descendant depth 2).
        const yOf = (id: string): number => out.nodes.find((n) => n.id === id)?.y ?? -1
        expect(yOf(hannelore)).toBe(yOf(otto))
        expect(yOf(greta)).toBe(yOf(werner))
        // No two distinct persons may occupy the SAME (x, y) — that's the
        // "card vanished" symptom (one rect covers another and the partner
        // hidden behind it is undetectable to the user).
        const xy = out.nodes.map((n) => `${n.x},${n.y}`)
        const dupes = xy.filter((s, i) => xy.indexOf(s) !== i)
        expect(dupes).toEqual([])
        // Adjacent same-row cards must be at least NODE_W apart to avoid
        // visual overlap.
        const byY = new Map<number, { id: string; x: number }[]>()
        for (const n of out.nodes) {
            const row = byY.get(n.y) ?? []
            row.push({ id: n.id, x: n.x })
            byY.set(n.y, row)
        }
        for (const row of byY.values()) {
            row.sort((p, q) => p.x - q.x)
            for (let i = 1; i < row.length; i += 1) {
                const a = row[i - 1]
                const b = row[i]
                if (a === undefined || b === undefined) continue
                expect(b.x - a.x).toBeGreaterThanOrEqual(NODE_W)
            }
        }
    })

    // "How does this hold up at scale" smoke test. Generates a synthetic
    // family of 50 generations of 20 siblings each (1000 persons) with
    // intra-generation partner pairing and parent_links to the prior
    // generation. We don't care about the visual outcome — only:
    //   1. layoutTree returns every input person.
    //   2. No two persons share an (x, y) on the canvas.
    //   3. Same-row adjacent cards are at least NODE_W apart.
    //   4. The function completes in well under a second on a dev machine.
    //
    // If this assertion ever flips, it usually means the partner pass
    // collided two pairs onto the same midpoint (the same shape of bug as
    // the "grandparent drops Hannelore" regression).
    it('lays out a 1000-person family without overlap or drop', () => {
        const generations = 50
        const perGen = 20
        const nodes: TreeInput['nodes'] = []
        const parentEdges: TreeInput['parent_edges'] = []
        const partnerEdges: TreeInput['partner_edges'] = []
        for (let g = 0; g < generations; g += 1) {
            for (let i = 0; i < perGen; i += 1) {
                const id = `g${g}-${i}`
                const parents: string[] = []
                if (g > 0) {
                    // Canonical parent: same column in prior gen. Plus a
                    // partner-style co-parent from the adjacent column.
                    parents.push(`g${g - 1}-${i}`)
                    parents.push(`g${g - 1}-${(i + 1) % perGen}`)
                }
                const partners = i % 2 === 0 ? [`g${g}-${i + 1}`] : []
                nodes.push(person(id, parents, partners, `${1900 + g * 2}-01-01`))
                for (const p of parents) {
                    parentEdges.push({ a: id, b: p })
                }
                if (i % 2 === 0 && i + 1 < perGen) {
                    partnerEdges.push({ a: id, b: `g${g}-${i + 1}` })
                }
            }
        }
        const start = performance.now()
        const out = layoutTree({ nodes, parent_edges: parentEdges, partner_edges: partnerEdges })
        const elapsed = performance.now() - start
        // 1. No dropped persons.
        expect(out.nodes).toHaveLength(generations * perGen)
        // 2. No (x, y) duplicates.
        const xy = out.nodes.map((n) => `${n.x},${n.y}`)
        const dupes = xy.filter((s, i) => xy.indexOf(s) !== i)
        expect(dupes).toEqual([])
        // 3. Per-row spacing ≥ NODE_W.
        const byY = new Map<number, number[]>()
        for (const n of out.nodes) {
            const row = byY.get(n.y) ?? []
            row.push(n.x)
            byY.set(n.y, row)
        }
        for (const row of byY.values()) {
            row.sort((a, b) => a - b)
            for (let i = 1; i < row.length; i += 1) {
                const a = row[i - 1]
                const b = row[i]
                if (a === undefined || b === undefined) continue
                expect(b - a).toBeGreaterThanOrEqual(NODE_W)
            }
        }
        // 4. Loose perf budget. 500ms is plenty; we just don't want a
        // quadratic regression sneaking past CI.
        expect(elapsed).toBeLessThan(500)
    })
})

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
            // Uniform spacing — rows are evenly stacked.
            const step = NODE_H + 100
            expect(gp - ggp).toBe(step)
            expect(p - gp).toBe(step)
            expect(c - p).toBe(step)
        }
    })

    it('eldest orphan with much older birth_date sits ABOVE the youngest top-row member', () => {
        // Two existing top-row people (one canonical-parent, one parentless)
        // and one true orphan whose birth date predates them by decades.
        // Pre-fix: all three share y=0 (depth-1 children of virtual root).
        // Post-fix: gp + grand-orphan-sibling sit on row 1, the 1910 orphan
        // is bumped above them.
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
            // The 1910 person sits ABOVE (smaller y means higher up on the
            // canvas) the 1936/1938 top row.
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
            // Older person above means smaller y on the SVG canvas.
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
        // Both end up on the same row (gen 0). No throw, no NaN, no Infinity.
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

    // v2: a couple's children sit centered on the couple's midpoint.
    it("centers a couple's children cluster on the couple midpoint", () => {
        // Two same-row partners with three children. Pre-v2 the children's
        // d3-tree x would have wandered. Post-v2 the children block-cluster
        // midpoint == the couple midpoint (within a half-COL_GAP slack to
        // account for odd-child counts producing a centered middle child).
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
            // Partners adjacent on the same row.
            expect(dad.y).toBe(mom.y)
            expect(Math.abs(dad.x - mom.x)).toBe(NODE_W + COL_GAP)
            const coupleMid = (Math.min(dad.x, mom.x) + Math.max(dad.x, mom.x) + NODE_W) / 2
            // Leftmost child's left edge and rightmost child's right edge
            // bracket the children-cluster. Its midpoint should match the
            // couple's midpoint.
            const childMid = (k1.x + 0 + (k3.x + NODE_W)) / 2
            expect(Math.abs(childMid - coupleMid)).toBeLessThanOrEqual(1)
        }
    })

    // v2: partners always adjacent (Δx == NODE_W + COL_GAP). Repro of the
    // user-reported "Werner – Hannelore – Greta – Otto" visual bug.
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
        // Each couple sits adjacent on the same row.
        expect(dx('otto', 'hannelore')).toBe(NODE_W + COL_GAP)
        expect(dx('werner', 'greta')).toBe(NODE_W + COL_GAP)
        expect(dx('klaus', 'anna')).toBe(NODE_W + COL_GAP)
        // Row y matches inside each couple.
        const yOf = (id: string): number => out.nodes.find((n) => n.id === id)?.y ?? -1
        expect(yOf('otto')).toBe(yOf('hannelore'))
        expect(yOf('werner')).toBe(yOf('greta'))
    })

    // v2: between two sibling clusters (different parent blocks) the gap
    // must be ≥ CLUSTER_GAP; inside one cluster it stays at COL_GAP.
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
            // Right child's left edge - left child's right edge >= CLUSTER_GAP.
            const gap = right.x - (left.x + NODE_W)
            expect(gap).toBeGreaterThanOrEqual(CLUSTER_GAP)
        }
    })

    // v2: inside one sibling group, gap stays at COL_GAP between adjacent
    // singletons. Same parents → siblings → tight COL_GAP.
    it('uses COL_GAP between siblings inside one cluster', () => {
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
            // Siblings: tight COL_GAP between them. CLUSTER_GAP gap would
            // mean we're failing to recognise them as same-cluster.
            expect(gap).toBe(COL_GAP)
        }
    })

    // v2: children sort left-to-right by birth date (oldest first, missing
    // trails). Birth-year ties fall back to the full ISO string.
    it('sorts siblings left-to-right by birth date', () => {
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
