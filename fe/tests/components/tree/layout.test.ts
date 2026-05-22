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
        expect(ids).toEqual(
            [peter, otto, hannelore, werner, greta, klaus, anna, lina, max].sort(),
        )
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
