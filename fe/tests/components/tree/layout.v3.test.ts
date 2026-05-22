// v3 layout invariants: top-down generation rank from parentless anchors
// + partner equalization. Each set here repros a user-reported bug from
// the expanded 20-person seed:
//   - Bug 1: Felix on the wrong row. A parent's biological+step kids
//     should ALL share one row regardless of whether each child has
//     descendants of their own.
//   - Bug 2: Brigitte on the wrong row. A root partner (no parent_links)
//     of a non-root partner should share the non-root partner's row.
//   - Regression: peter old must still sit above Otto's row.
//
// The shared `layout.regression.test.ts` continues to guard the 1000-person
// stress + the "no card vanishes" invariants — those still pass under v3.
import { describe, expect, it } from 'vitest'

import { layoutTree, type TreeInput } from '@/components/tree/layout'

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

describe('layoutTree (v3: top-down + partner equalize)', () => {
    it('sibling alignment: childless and child-having kids share their parent row', () => {
        // Parent `p` has TWO biological children:
        //   - `childless` is a leaf (no children of their own).
        //   - `parented` has a child `grand`.
        // Pre-v3 bottom-up depth gave `childless` gen 0 and `parented` gen 1,
        // so they landed on different rows even though they're siblings.
        // v3 places both at depth 1 (= parent.depth + 1).
        const out = layoutTree({
            nodes: [person('p'), person('childless', ['p']), person('parented', ['p']), person('grand', ['parented'])],
            parent_edges: [
                { a: 'childless', b: 'p' },
                { a: 'parented', b: 'p' },
                { a: 'grand', b: 'parented' },
            ],
            partner_edges: [],
        })
        const yOf = (id: string): number | undefined => out.nodes.find((n) => n.id === id)?.y
        expect(yOf('childless')).toBe(yOf('parented'))
        const py = yOf('p')
        const cy = yOf('childless')
        const gy = yOf('grand')
        expect(py).toBeDefined()
        expect(cy).toBeDefined()
        expect(gy).toBeDefined()
        if (py !== undefined && cy !== undefined && gy !== undefined) {
            expect(py).toBeLessThan(cy)
            expect(cy).toBeLessThan(gy)
        }
    })

    it('partner equalization: root partner of a non-root partner shares the non-root row', () => {
        // `klaus` has parents (depth 2). `brigitte` is a root partner (depth
        // 0 from the parent-edge pass). Partner equalization pulls Brigitte
        // up to klaus's row — Bug 2 in the user's report.
        const out = layoutTree({
            nodes: [
                person('gp1'),
                person('gp2', [], ['gp1']),
                person('klaus', ['gp1', 'gp2'], ['brigitte']),
                person('brigitte', [], ['klaus']),
            ],
            parent_edges: [
                { a: 'klaus', b: 'gp1' },
                { a: 'klaus', b: 'gp2' },
            ],
            partner_edges: [
                { a: 'gp1', b: 'gp2' },
                { a: 'klaus', b: 'brigitte' },
            ],
        })
        const yOf = (id: string): number | undefined => out.nodes.find((n) => n.id === id)?.y
        expect(yOf('klaus')).toBe(yOf('brigitte'))
        const gpY = yOf('gp1')
        const kY = yOf('klaus')
        expect(gpY).toBeDefined()
        expect(kY).toBeDefined()
        if (gpY !== undefined && kY !== undefined) {
            expect(gpY).toBeLessThan(kY)
        }
    })

    it('half-siblings sit on one row when only one parent is shared', () => {
        // The Felix half-sibling configuration: `klaus` (depth 2) is shared
        // by Felix (other parent `brigitte`, root) and Lina/Max (other parent
        // `anna`, depth 2 via her own parents). Partner equalization makes
        // Brigitte/Klaus/Anna all depth 2, so Felix/Lina/Max all sit at depth 3.
        // Lina has children Emma/Noah — they push the maxDepth without
        // affecting Felix's row.
        const out = layoutTree({
            nodes: [
                person('otto'),
                person('hannelore', [], ['otto']),
                person('werner'),
                person('greta', [], ['werner']),
                person('klaus', ['otto', 'hannelore'], ['anna', 'brigitte']),
                person('anna', ['werner', 'greta'], ['klaus']),
                person('brigitte', [], ['klaus']),
                person('felix', ['klaus', 'brigitte', 'anna']),
                person('lina', ['klaus', 'anna'], [], '1995'),
                person('max', ['klaus', 'anna'], [], '1998'),
                person('emma', ['lina']),
                person('noah', ['lina']),
            ],
            parent_edges: [
                { a: 'klaus', b: 'otto' },
                { a: 'klaus', b: 'hannelore' },
                { a: 'anna', b: 'werner' },
                { a: 'anna', b: 'greta' },
                { a: 'felix', b: 'klaus' },
                { a: 'felix', b: 'brigitte' },
                { a: 'felix', b: 'anna' },
                { a: 'lina', b: 'klaus' },
                { a: 'lina', b: 'anna' },
                { a: 'max', b: 'klaus' },
                { a: 'max', b: 'anna' },
                { a: 'emma', b: 'lina' },
                { a: 'noah', b: 'lina' },
            ],
            partner_edges: [
                { a: 'otto', b: 'hannelore' },
                { a: 'werner', b: 'greta' },
                { a: 'klaus', b: 'anna' },
                { a: 'klaus', b: 'brigitte' },
            ],
        })
        const yOf = (id: string): number | undefined => out.nodes.find((n) => n.id === id)?.y
        // Felix, Lina, Max all on one row.
        expect(yOf('felix')).toBe(yOf('lina'))
        expect(yOf('felix')).toBe(yOf('max'))
        // Klaus, Anna, Brigitte all on one row above.
        expect(yOf('klaus')).toBe(yOf('anna'))
        expect(yOf('klaus')).toBe(yOf('brigitte'))
        // Otto/Hannelore/Werner/Greta all on one row above Klaus.
        expect(yOf('otto')).toBe(yOf('hannelore'))
        expect(yOf('werner')).toBe(yOf('greta'))
        expect(yOf('otto')).toBe(yOf('werner'))
        const oY = yOf('otto')
        const kY = yOf('klaus')
        const fY = yOf('felix')
        const eY = yOf('emma')
        expect(oY).toBeDefined()
        expect(kY).toBeDefined()
        expect(fY).toBeDefined()
        expect(eY).toBeDefined()
        if (oY !== undefined && kY !== undefined && fY !== undefined && eY !== undefined) {
            expect(oY).toBeLessThan(kY)
            expect(kY).toBeLessThan(fY)
            expect(fY).toBeLessThan(eY)
        }
    })

    it('peter old still sits above Otto when wired in as Otto parent', () => {
        // Repro mirrors the seed: peter old (1910) is otto's parent; otto
        // has descendants reaching depth 4 (Emma/Noah). peter ends up on
        // a row strictly above otto/hannelore/werner/greta.
        const out = layoutTree({
            nodes: [
                person('peter', [], [], '1910'),
                person('otto', ['peter'], ['hannelore'], '1935'),
                person('hannelore', [], ['otto'], '1938'),
                person('werner', [], ['greta'], '1936'),
                person('greta', [], ['werner'], '1940'),
                person('klaus', ['otto', 'hannelore'], ['anna'], '1965'),
                person('anna', ['werner', 'greta'], ['klaus'], '1968'),
                person('lina', ['klaus', 'anna'], [], '1995'),
                person('emma', ['lina'], [], '2020'),
            ],
            parent_edges: [
                { a: 'otto', b: 'peter' },
                { a: 'klaus', b: 'otto' },
                { a: 'klaus', b: 'hannelore' },
                { a: 'anna', b: 'werner' },
                { a: 'anna', b: 'greta' },
                { a: 'lina', b: 'klaus' },
                { a: 'lina', b: 'anna' },
                { a: 'emma', b: 'lina' },
            ],
            partner_edges: [
                { a: 'otto', b: 'hannelore' },
                { a: 'werner', b: 'greta' },
                { a: 'klaus', b: 'anna' },
            ],
        })
        const yOf = (id: string): number | undefined => out.nodes.find((n) => n.id === id)?.y
        const peterY = yOf('peter')
        const ottoY = yOf('otto')
        const wernerY = yOf('werner')
        expect(peterY).toBeDefined()
        expect(ottoY).toBeDefined()
        expect(wernerY).toBeDefined()
        if (peterY !== undefined && ottoY !== undefined && wernerY !== undefined) {
            expect(peterY).toBeLessThan(ottoY)
            expect(peterY).toBeLessThan(wernerY)
        }
    })

    it('full seeded 20-person family lays out without per-row mismatches', () => {
        // Spot-check the regression that motivated v3:
        //   - Felix on G3 (depth 3) alongside Lina/Max/Mia.
        //   - Brigitte on G2 alongside Klaus/Anna.
        //   - Emma/Noah on G4 (depth 4) under Lina.
        const out = layoutTree({
            nodes: [
                person('otto', [], ['hannelore'], '1935'),
                person('hannelore', [], ['otto'], '1938'),
                person('werner', [], ['greta'], '1936'),
                person('greta', [], ['werner'], '1940'),
                person('friedrich', [], ['lotte'], '1932'),
                person('lotte', [], ['friedrich'], '1934'),
                person('klaus', ['otto', 'hannelore'], ['anna', 'brigitte'], '1965'),
                person('anna', ['werner', 'greta'], ['klaus'], '1968'),
                person('brigitte', [], ['klaus'], '1968'),
                person('sabine', ['werner', 'greta'], ['julia'], '1970'),
                person('julia', [], ['sabine'], '1972'),
                person('markus', ['friedrich', 'lotte'], [], '1967'),
                person('felix', ['klaus', 'brigitte', 'anna'], [], '1992'),
                person('lina', ['klaus', 'anna'], [], '1995'),
                person('max', ['klaus', 'anna'], [], '1998'),
                person('mia', ['klaus', 'anna'], [], '2001'),
                person('lena', ['sabine', 'julia'], [], '2005'),
                person('tom', ['markus'], [], '1996'),
                person('emma', ['lina'], [], '2020'),
                person('noah', ['lina'], [], '2022'),
            ],
            parent_edges: [
                { a: 'klaus', b: 'otto' },
                { a: 'klaus', b: 'hannelore' },
                { a: 'anna', b: 'werner' },
                { a: 'anna', b: 'greta' },
                { a: 'sabine', b: 'werner' },
                { a: 'sabine', b: 'greta' },
                { a: 'markus', b: 'friedrich' },
                { a: 'markus', b: 'lotte' },
                { a: 'felix', b: 'klaus' },
                { a: 'felix', b: 'brigitte' },
                { a: 'felix', b: 'anna' },
                { a: 'lina', b: 'klaus' },
                { a: 'lina', b: 'anna' },
                { a: 'max', b: 'klaus' },
                { a: 'max', b: 'anna' },
                { a: 'mia', b: 'klaus' },
                { a: 'mia', b: 'anna' },
                { a: 'lena', b: 'sabine' },
                { a: 'lena', b: 'julia' },
                { a: 'tom', b: 'markus' },
                { a: 'emma', b: 'lina' },
                { a: 'noah', b: 'lina' },
            ],
            partner_edges: [
                { a: 'otto', b: 'hannelore' },
                { a: 'werner', b: 'greta' },
                { a: 'klaus', b: 'anna' },
                { a: 'klaus', b: 'brigitte' },
                { a: 'sabine', b: 'julia' },
                { a: 'friedrich', b: 'lotte' },
            ],
        })
        const yOf = (id: string): number => out.nodes.find((n) => n.id === id)?.y ?? Number.NaN
        // G3: Felix shares the row with Lina, Max, Mia.
        expect(yOf('felix')).toBe(yOf('lina'))
        expect(yOf('felix')).toBe(yOf('max'))
        expect(yOf('felix')).toBe(yOf('mia'))
        // G2: Klaus, Anna, Brigitte on one row.
        expect(yOf('klaus')).toBe(yOf('anna'))
        expect(yOf('klaus')).toBe(yOf('brigitte'))
        // G2: Sabine + Julia on the partner row too (both equalized to depth 2).
        expect(yOf('sabine')).toBe(yOf('julia'))
        expect(yOf('sabine')).toBe(yOf('klaus'))
        // G1 grandparent row (Otto/Hannelore/Werner/Greta) is one row above G2.
        expect(yOf('otto')).toBe(yOf('hannelore'))
        expect(yOf('werner')).toBe(yOf('greta'))
        expect(yOf('otto')).toBe(yOf('werner'))
        // Emma + Noah on G4, strictly below G3.
        expect(yOf('emma')).toBe(yOf('noah'))
        expect(yOf('emma')).toBeGreaterThan(yOf('felix'))
    })
})
