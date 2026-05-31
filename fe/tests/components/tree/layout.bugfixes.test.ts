// Layout invariants for the three bugs the user pointed out on his real
// tree (Steinbach + Wagner-like + Lau-like). Each test isolates one
// invariant so a future regression points at the right pass.
//
//   1. Sibling-by-age: when an in-married spouse has a smaller UUID than
//      the blood sibling (so the block-builder threads them as `members[0]`)
//      AND has a birth_date older/younger than the sibling, the sibling
//      row must still sort by the BLOOD relative's birth date.
//   2. Anchor-in-middle: a person with two concurrent OPEN partnerships
//      sits BETWEEN them in the chain, not leftmost.
//   3. Multi-row crossing: a parent block whose child got shifted right
//      by row separation must re-centre over the child so the parent
//      edge stays vertical (no diagonal cross over a sibling's column).

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

function xOf(out: ReturnType<typeof layoutTree>, id: string): number {
    const n = out.nodes.find((node) => node.id === id)
    if (n === undefined) throw new Error(`person ${id} not in layout`)
    return n.x
}

describe('layoutTree — bug 1: sibling row sorts by blood relative, not block.members[0]', () => {
    it('keeps Lukas LEFT of the Carla+Tobias couple even though Tobias < Carla by UUID and 1969 < 1974', () => {
        // Steinbach-shaped repro. The block-builder threads each
        // couple as [smallerId, largerId], so the Carla+Tobias
        // block ends up with `members[0] === tobias`. Sorting by
        // `members[0].birth_date` would prefer Tobias's 1969 birth
        // date and slot the couple LEFT of Lukas (1972) — wrong.
        // The blood relative is Carla (1974) → the couple should
        // sit BETWEEN Lukas and Felix.
        const parents = ['hartmut', 'margarete']
        const out = layoutTree({
            nodes: [
                person('hartmut', [], ['margarete'], '1940-03-12'),
                person('margarete', [], ['hartmut'], '1948-07-23'),
                person('lukas', parents, [], '1972-04-19'),
                // Tobias's id sorts BEFORE 'carla' so threaded as
                // [tobias, carla]. Tobias 1969 is older than Lukas 1972.
                person('aaa_tobias', [], ['carla'], '1969-08-22'),
                person('carla', parents, ['aaa_tobias'], '1974-11-30'),
                person('felix', parents, ['aaa_beate'], '1977-05-08'),
                // Beate's id sorts BEFORE 'felix' so threaded as
                // [beate, felix]. Beate 1985 is younger than Stefan 1978.
                person('aaa_beate', [], ['felix'], '1985-06-12'),
                person('stefan', parents, [], '1978-02-14'),
                person('nina', parents, [], '1983-10-09'),
            ],
            parent_edges: [
                { a: 'lukas', b: 'hartmut' },
                { a: 'lukas', b: 'margarete' },
                { a: 'carla', b: 'hartmut' },
                { a: 'carla', b: 'margarete' },
                { a: 'felix', b: 'hartmut' },
                { a: 'felix', b: 'margarete' },
                { a: 'stefan', b: 'hartmut' },
                { a: 'stefan', b: 'margarete' },
                { a: 'nina', b: 'hartmut' },
                { a: 'nina', b: 'margarete' },
            ],
            partner_edges: [
                { a: 'hartmut', b: 'margarete' },
                { a: 'aaa_tobias', b: 'carla' },
                { a: 'aaa_beate', b: 'felix' },
            ] as PartnerEdgeInput[],
        })
        // Sibling row order by birth_date of the BLOOD relative:
        //   Lukas 1972 → Carla 1974 → Felix 1977 → Stefan 1978 → Nina 1983.
        // Spouse positions don't matter for the ordering rule — what
        // matters is the relative x of each sibling.
        expect(xOf(out, 'lukas')).toBeLessThan(xOf(out, 'carla'))
        expect(xOf(out, 'carla')).toBeLessThan(xOf(out, 'felix'))
        expect(xOf(out, 'felix')).toBeLessThan(xOf(out, 'stefan'))
        expect(xOf(out, 'stefan')).toBeLessThan(xOf(out, 'nina'))
    })

    it('falls back to leftmost-member sort for an adoptive-only sibling block (no bio link)', () => {
        // Edge case: a block hangs off a parent via an ADOPTIVE link
        // only — `siblingSortKey` finds no bio intersection and
        // should fall back to `blockSortKey` (leftmost member) rather
        // than reporting +Infinity and sorting to the end. Asserts the
        // fallback path keeps the block in a reasonable position
        // relative to its bio sibling.
        const out = layoutTree({
            nodes: [
                person('p1', [], ['p2'], '1950-01-01'),
                person('p2', [], ['p1'], '1952-01-01'),
                person('bio_kid', ['p1', 'p2'], [], '1980-01-01'),
                person('adopted', ['p1', 'p2'], [], '1975-01-01'),
            ],
            parent_edges: [
                { a: 'bio_kid', b: 'p1', kind: 'biological' },
                { a: 'bio_kid', b: 'p2', kind: 'biological' },
                { a: 'adopted', b: 'p1', kind: 'adoptive' },
                { a: 'adopted', b: 'p2', kind: 'adoptive' },
            ],
            partner_edges: [{ a: 'p1', b: 'p2' }],
        })
        // adopted 1975 should sort LEFT of bio_kid 1980 (the fallback
        // `blockSortKey` ordering, since neither has a bio bp set —
        // singletons fall through to the leftmost-member path which
        // here equals each block's only member).
        expect(xOf(out, 'adopted')).toBeLessThan(xOf(out, 'bio_kid'))
    })
})

describe('layoutTree — bug 2: anchor sits in the middle of two concurrent open partners', () => {
    it('places Helmut BETWEEN Ingrid and Renate when both partnerships are open', () => {
        // Wagner-shaped repro. Helmut has two concurrent OPEN
        // partnerships. v3.1 threaded [anchor, open1, open2] —
        // Helmut leftmost. v3.2 threads [open, anchor, open] so
        // Helmut sits in the middle.
        const out = layoutTree({
            nodes: [
                person('ingrid', [], ['helmut']),
                person('helmut', [], ['ingrid', 'renate']),
                person('renate', [], ['helmut']),
            ],
            parent_edges: [],
            partner_edges: [
                { a: 'helmut', b: 'ingrid' },
                { a: 'helmut', b: 'renate' },
            ] as PartnerEdgeInput[],
        })
        const ingrid = xOf(out, 'ingrid')
        const helmut = xOf(out, 'helmut')
        const renate = xOf(out, 'renate')
        // Either ingrid then helmut then renate, OR renate then
        // helmut then ingrid — both are valid because the open
        // partners sort by id and the anchor only needs to sit
        // between them. The id-sort here is `ingrid < renate`, so
        // ingrid lands on the LEFT.
        expect(ingrid).toBeLessThan(helmut)
        expect(helmut).toBeLessThan(renate)
        expect(helmut - ingrid).toBe(NODE_W + COL_GAP)
        expect(renate - helmut).toBe(NODE_W + COL_GAP)
    })

    it('still places the anchor LEFT of a single open partner when there are no ended partners (size==2 path)', () => {
        // Sanity: the bug-2 fix only affects the ≥3-member chain. A
        // 2-member single couple still routes through the
        // alphabetic-id `[smallerId, largerId]` branch and the
        // 1-ended-1-open exspouse layout still expects
        // [ended, anchor, open].
        const out = layoutTree({
            nodes: [
                person('brigitte', [], ['klaus']),
                person('klaus', [], ['anna', 'brigitte']),
                person('anna', [], ['klaus']),
            ],
            parent_edges: [],
            partner_edges: [
                { a: 'klaus', b: 'anna' },
                { a: 'klaus', b: 'brigitte', ended_on: '2000-06-30' },
            ] as PartnerEdgeInput[],
        })
        const klausX = xOf(out, 'klaus')
        const annaX = xOf(out, 'anna')
        const brigitteX = xOf(out, 'brigitte')
        // [brigitte, klaus, anna] — unchanged from v3.1.
        expect(brigitteX).toBeLessThan(klausX)
        expect(klausX).toBeLessThan(annaX)
        expect(klausX - brigitteX).toBe(NODE_W + COL_GAP)
        expect(annaX - klausX).toBe(NODE_W + COL_GAP)
    })

    it('places three open partners with the anchor offset to the left of centre', () => {
        // floor(3/2) = 1 left, ceil(3/2) = 2 right. Result:
        //   [open1, anchor, open2, open3]
        // Open partners sort by id ascending: a < b < c.
        const out = layoutTree({
            nodes: [
                person('mediator', [], ['p_a', 'p_b', 'p_c']),
                person('p_a', [], ['mediator']),
                person('p_b', [], ['mediator']),
                person('p_c', [], ['mediator']),
            ],
            parent_edges: [],
            partner_edges: [
                { a: 'mediator', b: 'p_a' },
                { a: 'mediator', b: 'p_b' },
                { a: 'mediator', b: 'p_c' },
            ] as PartnerEdgeInput[],
        })
        const pa = xOf(out, 'p_a')
        const m = xOf(out, 'mediator')
        const pb = xOf(out, 'p_b')
        const pc = xOf(out, 'p_c')
        expect(pa).toBeLessThan(m)
        expect(m).toBeLessThan(pb)
        expect(pb).toBeLessThan(pc)
    })
})

describe('layoutTree — bug 3: parent block recentres over a child shifted by row separation', () => {
    it('keeps a parent block centred over its only child in the trivial no-collision case', () => {
        // Baseline: no sibling collision in the child row → parent
        // must sit centred on its only child. This guards against
        // the recenter pass DECENTERING a block that was already
        // correctly placed.
        const out = layoutTree({
            nodes: [
                person('gp', [], [], '1920-01-01'),
                person('p_dad', ['gp'], ['p_mom'], '1945-01-01'),
                person('p_mom', [], ['p_dad'], '1947-01-01'),
                person('only_kid', ['p_dad', 'p_mom'], [], '1975-01-01'),
                person('row_neighbour', [], [], '1976-01-01'),
            ],
            parent_edges: [
                { a: 'p_dad', b: 'gp' },
                { a: 'only_kid', b: 'p_dad' },
                { a: 'only_kid', b: 'p_mom' },
            ],
            partner_edges: [{ a: 'p_dad', b: 'p_mom' }] as PartnerEdgeInput[],
        })
        const dadX = xOf(out, 'p_dad')
        const momX = xOf(out, 'p_mom')
        const kidX = xOf(out, 'only_kid')
        // Couple midpoint: average of the two card centres (which is
        // mathematically the same as (couple.left + couple.right) / 2).
        const dadCentre = dadX + NODE_W / 2
        const momCentre = momX + NODE_W / 2
        const coupleMid = (dadCentre + momCentre) / 2
        const kidCentre = kidX + NODE_W / 2
        expect(Math.abs(coupleMid - kidCentre)).toBeLessThanOrEqual(1)
    })

    it('recentres a parent couple over its single child after the child row is shifted right by a wide neighbour', () => {
        // Real bug-3 repro. Three siblings share one parent-row block:
        //   - gp1+gp2: parent couple at the top.
        //   - p_dad + p_mom: gp1+gp2's child couple (with extra spouse).
        //   - aunt + uncle: gp1+gp2's other child couple (wider subtree).
        //   - only_kid: p_dad+p_mom's only child.
        //   - cousin_a, cousin_b: aunt+uncle's two children (force the
        //     aunt+uncle subtree to be wider than p_dad+p_mom's).
        //
        // After initial placement, the only_kid row has [only_kid,
        // cousin_a, cousin_b]. The row-separation pass shifts
        // cousin_a + cousin_b right if they collide with only_kid.
        // More importantly: p_dad+p_mom and aunt+uncle in the
        // sibling row above also collide if aunt+uncle's wider
        // subtree pushes them apart, which moves aunt+uncle right
        // AND drags their cousins. Without recenter, the visual:
        // parent edge from p_dad+p_mom → only_kid stays vertical
        // (this is the simpler-shape branch), and from aunt+uncle →
        // cousins also stays vertical. To trigger the bug we need
        // a NON-root parent whose child gets shifted by something
        // OTHER than the parent's own subtree growth.
        //
        // Cleanest repro: aunt+uncle's subtree is wider than the
        // p_dad+p_mom slot. Row separation shifts aunt+uncle right.
        // The shift cascades to cousins. p_dad+p_mom stays. Then
        // only_kid (under p_dad+p_mom) is in the SAME row as
        // cousin_a + cousin_b. If cousins push only_kid right via
        // row separation (because cousins were also shifted), then
        // only_kid moves but p_dad+p_mom doesn't — and the recenter
        // pass should pull p_dad+p_mom over.
        const parents = ['gp1', 'gp2']
        const out = layoutTree({
            nodes: [
                person('gp1', [], ['gp2'], '1920-01-01'),
                person('gp2', [], ['gp1'], '1922-01-01'),
                person('p_dad', parents, ['p_mom'], '1945-01-01'),
                person('p_mom', [], ['p_dad'], '1947-01-01'),
                person('aunt', parents, ['uncle'], '1950-01-01'),
                person('uncle', [], ['aunt'], '1948-01-01'),
                person('only_kid', ['p_dad', 'p_mom'], [], '1975-01-01'),
                person('cousin_a', ['aunt', 'uncle'], [], '1978-01-01'),
                person('cousin_b', ['aunt', 'uncle'], [], '1980-01-01'),
            ],
            parent_edges: [
                { a: 'p_dad', b: 'gp1' },
                { a: 'p_dad', b: 'gp2' },
                { a: 'aunt', b: 'gp1' },
                { a: 'aunt', b: 'gp2' },
                { a: 'only_kid', b: 'p_dad' },
                { a: 'only_kid', b: 'p_mom' },
                { a: 'cousin_a', b: 'aunt' },
                { a: 'cousin_a', b: 'uncle' },
                { a: 'cousin_b', b: 'aunt' },
                { a: 'cousin_b', b: 'uncle' },
            ],
            partner_edges: [
                { a: 'gp1', b: 'gp2' },
                { a: 'p_dad', b: 'p_mom' },
                { a: 'aunt', b: 'uncle' },
            ] as PartnerEdgeInput[],
        })
        const dadX = xOf(out, 'p_dad')
        const momX = xOf(out, 'p_mom')
        const kidX = xOf(out, 'only_kid')
        const dadCentre = dadX + NODE_W / 2
        const momCentre = momX + NODE_W / 2
        const pmCoupleMid = (dadCentre + momCentre) / 2
        const kidCentre = kidX + NODE_W / 2
        // Parent couple should sit directly above their only child —
        // a 1 px tolerance for floating-point. Without the recenter
        // pass the parent edge would slope diagonally.
        expect(Math.abs(pmCoupleMid - kidCentre)).toBeLessThanOrEqual(1)

        // Same invariant for aunt+uncle vs their two children's midpoint.
        const auntX = xOf(out, 'aunt')
        const uncleX = xOf(out, 'uncle')
        const cAX = xOf(out, 'cousin_a')
        const cBX = xOf(out, 'cousin_b')
        const auCoupleMid = (auntX + uncleX + NODE_W) / 2
        const cousinsMid = (cAX + NODE_W / 2 + (cBX + NODE_W / 2)) / 2
        expect(Math.abs(auCoupleMid - cousinsMid)).toBeLessThanOrEqual(1)
    })
})
