// Regression repros for the SVG layout. Kept in a separate file so we can
// keep adding cases without hitting the 500-line cap on `layout.test.ts`.
import { describe, expect, it } from 'vitest'

import { layoutTree, NODE_W, type TreeInput } from '@/components/tree/layout'

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
// payload still has all 9 persons; the bug was purely in layoutTree.
describe('layoutTree (regression: grandparent does not drop nodes)', () => {
    it('returns all 9 persons in the seeded family + added grandparent', () => {
        const peter = 'c836a69f-bd6e-42df-abc6-50ceab0f815d'
        const otto = '00000003-0000-0000-0000-000000000001'
        const hannelore = '00000003-0000-0000-0000-000000000002'
        const werner = '00000003-0000-0000-0000-000000000003'
        const greta = '00000003-0000-0000-0000-000000000004'
        const klaus = '00000003-0000-0000-0000-000000000005'
        const anna = '00000003-0000-0000-0000-000000000006'
        const lina = '00000003-0000-0000-0000-000000000007'
        const max = '00000003-0000-0000-0000-000000000008'
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
        expect(out.nodes).toHaveLength(generations * perGen)
        const xy = out.nodes.map((n) => `${n.x},${n.y}`)
        const dupes = xy.filter((s, i) => xy.indexOf(s) !== i)
        expect(dupes).toEqual([])
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
        expect(elapsed).toBeLessThan(500)
    })
})
