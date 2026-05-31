// Pinned layout edge cases from a real family tree (Brüggemann tree,
// screenshot dated 2026-05-31). These assertions cover the two global
// layout rules:
//
//   1. Siblings sort by birthdate, oldest LEFT → youngest RIGHT, and
//      adding a spouse to one sibling must NOT shuffle the sibling row.
//   2. Same-row parent persons / in-married couples should be ordered
//      to AVOID crossing parent / partner edges with the row below
//      whenever a non-crossing order exists.
//
// Implementation note: the layout uses a two-pass barycenter heuristic
// (see `index.ts::reorderRootsByBarycenter` + `swapTwoPersonCouplesByParentX`)
// — first pass produces a default placement, the second pass re-sorts
// roots by descendant centre-of-mass and swaps in-married couple
// members whose parents sit on opposite sides of the row above.

import { describe, expect, it } from 'vitest'

import { layoutTree, type TreeInput } from '@/components/tree/layout'

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

function xOf(out: ReturnType<typeof layoutTree>, id: string): number {
    const n = out.nodes.find((node) => node.id === id)
    if (n === undefined) throw new Error(`person ${id} not in layout`)
    return n.x
}

describe('layoutTree — barycenter passes for the Brüggemann tree edge cases', () => {
    it('keeps three siblings in birth-date order when the youngest gains a spouse', () => {
        // Three children of one parent couple. Adding a spouse to the
        // youngest must NOT push the resulting couple block out of the
        // right-end position of the sibling row.
        const out = layoutTree({
            nodes: [
                person('p1', [], ['p2'], '1955-01-01'),
                person('p2', [], ['p1'], '1957-01-01'),
                person('lars', ['p1', 'p2'], [], '1985-01-14'),
                person('marie', ['p1', 'p2'], [], '1987-06-15'),
                person('tim', ['p1', 'p2'], ['mia'], '1989-03-22'),
                person('mia', [], ['tim'], '1988-05-10'),
            ],
            parent_edges: [
                { a: 'lars', b: 'p1' },
                { a: 'lars', b: 'p2' },
                { a: 'marie', b: 'p1' },
                { a: 'marie', b: 'p2' },
                { a: 'tim', b: 'p1' },
                { a: 'tim', b: 'p2' },
            ],
            partner_edges: [
                { a: 'p1', b: 'p2' },
                { a: 'tim', b: 'mia' },
            ] as PartnerEdgeInput[],
        })
        expect(xOf(out, 'lars')).toBeLessThan(xOf(out, 'marie'))
        expect(xOf(out, 'marie')).toBeLessThan(xOf(out, 'tim'))
    })

    it('reorders top-row roots so each sits closer to its descendants (Krause: Anneliese LEFT, Greta RIGHT)', () => {
        // Greta is mother of Hubert; Anneliese is mother of Reinhardt.
        // Hubert + Sara have child Mia. Reinhardt + Helga have child Tim.
        // Tim + Mia are an in-married couple at the bottom — Tim's
        // siblings (none in this minimal fixture) AND Mia anchor the
        // bottom row. The barycenter pass should put Anneliese (whose
        // descendant Tim sits on the side of his parents Reinhardt +
        // Helga) on the same side as her descendants, and Greta on the
        // other side — eliminating the crossing the default
        // birth-date sort produced (Greta 1912 < Anneliese 1921 is the
        // default order; swapping by descendant barycenter fixes it).
        const out = layoutTree({
            nodes: [
                person('greta', [], [], '1912-03-29'),
                person('anneliese', [], [], '1921-03-25'),
                person('hubert', ['greta'], ['sara'], '1947-11-07'),
                person('sara', [], ['hubert'], '1956-11-22'),
                person('reinhardt', ['anneliese'], ['helga'], '1942-11-25'),
                person('helga', [], ['reinhardt'], '1958-09-12'),
                person('tim', ['reinhardt', 'helga'], ['mia'], '1989-03-22'),
                person('mia', ['hubert', 'sara'], ['tim'], '1988-05-10'),
            ],
            parent_edges: [
                { a: 'hubert', b: 'greta' },
                { a: 'reinhardt', b: 'anneliese' },
                { a: 'mia', b: 'hubert' },
                { a: 'mia', b: 'sara' },
                { a: 'tim', b: 'reinhardt' },
                { a: 'tim', b: 'helga' },
            ],
            partner_edges: [
                { a: 'hubert', b: 'sara' },
                { a: 'reinhardt', b: 'helga' },
                { a: 'tim', b: 'mia' },
            ] as PartnerEdgeInput[],
        })
        // Top-row roots: descendant centre-of-mass should put each above
        // its own line of descendants. Hubert's branch contains Mia;
        // Reinhardt's branch contains Tim. After the barycenter pass the
        // mothers stack in the same order as their descendants.
        const aX = xOf(out, 'anneliese')
        const gX = xOf(out, 'greta')
        const hX = xOf(out, 'hubert')
        const bX = xOf(out, 'reinhardt')
        // If Anneliese ends up above Reinhardt and Greta above Hubert,
        // the top-row order matches the middle-row order (no
        // crossings between the two top→middle parent edges).
        if (bX < hX) {
            expect(aX).toBeLessThan(gX)
        } else {
            expect(gX).toBeLessThan(aX)
        }
    })

    it('swaps an in-married couple to place each spouse closer to their own parents', () => {
        // Two parent couples on opposite sides of their row.
        // Their kids marry: arnd (left-couple's child) + marissa
        // (right-couple's child). The couple block's default member
        // order is alphabetical (arnd < marissa), which puts arnd on
        // the LEFT of the couple. With the right-couple to the RIGHT,
        // marissa's parent-edge would have to span back across arnd
        // to reach her parents. The swap pass detects this and
        // reverses the members so each spouse sits on the side of
        // their own parents.
        const out = layoutTree({
            nodes: [
                person('arnd_dad', [], ['arnd_mom'], '1942-11-25'),
                person('arnd_mom', [], ['arnd_dad'], '1958-09-12'),
                person('marissa_dad', [], ['marissa_mom'], '1947-11-07'),
                person('marissa_mom', [], ['marissa_dad'], '1956-11-22'),
                person('arnd', ['arnd_dad', 'arnd_mom'], ['marissa'], '1988-05-13'),
                person('marissa', ['marissa_dad', 'marissa_mom'], ['arnd'], '1986-02-08'),
            ],
            parent_edges: [
                { a: 'arnd', b: 'arnd_dad' },
                { a: 'arnd', b: 'arnd_mom' },
                { a: 'marissa', b: 'marissa_dad' },
                { a: 'marissa', b: 'marissa_mom' },
            ],
            partner_edges: [
                { a: 'arnd_dad', b: 'arnd_mom' },
                { a: 'marissa_dad', b: 'marissa_mom' },
                { a: 'arnd', b: 'marissa' },
            ] as PartnerEdgeInput[],
        })
        // Whichever parent couple ends up LEFT, that couple's child
        // must be on the LEFT half of the in-married couple.
        const arndDadX = xOf(out, 'arnd_dad')
        const marissaDadX = xOf(out, 'marissa_dad')
        const arndX = xOf(out, 'arnd')
        const marissaX = xOf(out, 'marissa')
        if (arndDadX < marissaDadX) {
            expect(arndX).toBeLessThan(marissaX)
        } else {
            expect(marissaX).toBeLessThan(arndX)
        }
    })
})
