// SVG layout for the family tree. v3: top-down generation rank from
// parentless anchors with partner equalization (see ./generations.ts),
// then the block-based layout from v2 — each "couple" (two same-row
// partners) is the placement unit. Children sit centered under their
// parent block; sibling groups of different parent blocks are separated
// by a wider CLUSTER_GAP so the visual grouping is unambiguous. Top-row
// blocks are laid out left-to-right by birth date in stable id order.
//
// v3.1 (ex-spouse adjacency): a person with both an open and an ended
// partnership on the same row gets ONE multi-member block instead of two
// disjoint slots. Ended partners thread to the LEFT of the shared anchor,
// open ones to the RIGHT; each sub-cluster of children centres under its
// own bio-couple midpoint.

import { blockSortKey, buildBlocks, chooseParentBlock, compareBlockKeys } from './blocks'
import { computeGenerations, promoteEldestOrphans } from './generations'
import { compactRootClusters } from './compact'
import { materializeLayout } from './materialize'
import { recenterParentsOverChildren, runRowSeparation } from './recenter'
import { reorderRootsByBarycenter, runPlacement, swapTwoPersonCouplesByParentX } from './reorder'
import { type Block, type LayoutResult, NODE_H, type PositionedBlock, ROW_GAP, type TreeInput } from './types'

export { type BackendNode, COL_GAP, type LayoutResult, NODE_H, NODE_W, type Positioned, type TreeInput } from './types'

/**
 * Compute SVG-ready positions and edge coordinates for the family tree.
 *
 * Strategy (v3.1):
 *   1. Build child-of-person AND parent-of-person adjacency from
 *      parent_edges. Track biological parents separately so a step-link
 *      doesn't pull a child under the wrong couple inside a multi-couple
 *      block. Compute the depth of every person top-down from parentless
 *      anchors, equalize partners + propagate upward to a fixed point,
 *      invert to a "gen" index (top row == max gen), and promote eldest
 *      orphans by birth-year gap.
 *   2. Build per-row blocks: each same-row partner-edge connected
 *      component becomes one block (singleton, couple, or N≥3 chain).
 *   3. Choose a canonical parent block for each non-top block (the block
 *      containing the smallest-id parent of the block's anchor member).
 *   4. Place top-row blocks left-to-right with CLUSTER_GAP between them.
 *      Recursively lay out each top block's subtree, with multi-couple
 *      blocks splitting their children into per-couple sub-clusters.
 *   5. Sweep each row for inter-cluster collisions and enforce COL_GAP
 *      within a cluster, CLUSTER_GAP between clusters belonging to
 *      different parent blocks. Uses each block's `pixelWidth` so widened
 *      multi-couple blocks still get correct separation.
 *   6. Materialize positioned persons by reading each block's per-member
 *      x offset. Emit parent + partner edges from those coordinates.
 */
export function layoutTree(input: TreeInput): LayoutResult {
    const byId = new Map(input.nodes.map((n) => [n.id, n]))

    // Full parent adjacency for the generation-rank pass (and for the
    // post-layout parent-edge render). EdgePair: `a` = child, `b` = parent.
    const childrenOfPerson = new Map<string, string[]>()
    const parentsOfPerson = new Map<string, string[]>()
    // Bio parents: edges with `kind === 'biological'` or unknown kind (the
    // unit-test fixtures elide the field). Used by `subtree.ts` to bucket
    // children of a multi-couple block under the correct bio couple.
    const bioParents = new Map<string, Set<string>>()
    for (const e of input.parent_edges) {
        if (!byId.has(e.a) || !byId.has(e.b)) continue
        const kids = childrenOfPerson.get(e.b) ?? []
        kids.push(e.a)
        childrenOfPerson.set(e.b, kids)
        const parents = parentsOfPerson.get(e.a) ?? []
        parents.push(e.b)
        parentsOfPerson.set(e.a, parents)
        const isBio = e.kind === undefined || e.kind === 'biological'
        if (isBio) {
            const bp = bioParents.get(e.a) ?? new Set<string>()
            bp.add(e.b)
            bioParents.set(e.a, bp)
        }
    }

    const validPartnerEdges = input.partner_edges.filter((e) => byId.has(e.a) && byId.has(e.b))

    const baseGeneration = computeGenerations(
        input.nodes.map((n) => n.id),
        parentsOfPerson,
        validPartnerEdges,
    )
    const generation = promoteEldestOrphans(input.nodes, baseGeneration, childrenOfPerson)
    let topGen = 0
    for (const g of generation.values()) if (g > topGen) topGen = g

    const blocksByRow = buildBlocks(
        input.nodes.map((n) => n.id),
        generation,
        validPartnerEdges,
    )

    // Block-by-person reverse index. Used to look up a block's parent block
    // (block containing the canonical parent person).
    const blockOfPerson = new Map<string, Block>()
    for (const list of blocksByRow.values()) {
        for (const b of list) {
            for (const m of b.members) blockOfPerson.set(m, b)
        }
    }

    // Tag every block with its y in pixel space (top row at y=0).
    const rowStep = NODE_H + ROW_GAP
    for (const [g, list] of blocksByRow.entries()) {
        const y = (topGen - g) * rowStep
        for (const b of list) b.y = y
    }

    // Build child-of-block adjacency. A block with no canonical parent
    // block becomes a "root" of its own subtree.
    const childrenOfBlock = new Map<string, Block[]>()
    const rootBlocks: Block[] = []
    for (const [g, list] of blocksByRow.entries()) {
        for (const b of list) {
            if (g === topGen) {
                rootBlocks.push(b)
                continue
            }
            const pb = chooseParentBlock(b, blockOfPerson, byId)
            if (pb === null) {
                rootBlocks.push(b)
                continue
            }
            const kids = childrenOfBlock.get(pb.id) ?? []
            kids.push(b)
            childrenOfBlock.set(pb.id, kids)
        }
    }

    rootBlocks.sort((a, b) => compareBlockKeys(blockSortKey(a, byId), blockSortKey(b, byId)))

    // Two-pass layout to eliminate avoidable parent / partner edge crossings:
    //   1. First pass uses the default ordering (root blocks sorted by their
    //      oldest member's birth date; 2-person couples sorted alphabetically
    //      by id) and gives us a first-cut position for every block.
    //   2. From the first-pass positions we compute, per root block, the
    //      descendant barycenter (mean x of all descendants) — that's the
    //      column the root WOULD naturally sit above if it had to balance
    //      the centre of mass below it. We then re-sort root blocks by
    //      barycenter so each ends up directly above its descendants instead
    //      of being forced into a birth-date order that drags the children
    //      sideways.
    //   3. Same idea for 2-person in-married couples that join two parent
    //      blocks on opposite sides of the row above. We compute each
    //      member's canonical parent-block x from pass 1; if reversing the
    //      members puts each one closer to their own parents, we swap.
    //   4. A second placement pass redraws everything against the new
    //      orderings. The function is idempotent — running pass 2 again on
    //      the new positions wouldn't change anything as long as no further
    //      swaps are needed.
    //
    // Targeted cases (see `upcoming-tree-layout-rules` memory + the Krause
    // subtree in the seed):
    //   - Two unpartnered top-row mothers whose children sit on opposite
    //     sides of the row below (Greta + Anneliese).
    //   - In-married couples whose spouses come from parent blocks on
    //     opposite sides (Tim + Mia).
    const placed = runPlacement(rootBlocks, childrenOfBlock, byId, bioParents)

    // Pass 2 reorderings driven by pass-1 positions.
    let changed = false
    if (reorderRootsByBarycenter(rootBlocks, placed, childrenOfPerson)) changed = true
    if (swapTwoPersonCouplesByParentX(blocksByRow, placed, byId, parentsOfPerson)) changed = true
    if (changed) {
        placed.clear()
        for (const next of runPlacement(rootBlocks, childrenOfBlock, byId, bioParents).entries()) {
            placed.set(next[0], next[1])
        }
    }

    // Per-row separation pass. The recursive layout guarantees no overlap
    // *within* a subtree, but two subtrees on the same row may collide if
    // a descendant cluster is wider than the available slot. Walk each row
    // left-to-right and enforce a hard floor between blocks: COL_GAP if
    // they share a parent block (siblings), CLUSTER_GAP otherwise.
    const parentOfBlock = new Map<string, string | null>()
    for (const [pid, kids] of childrenOfBlock.entries()) {
        for (const k of kids) parentOfBlock.set(k.id, pid)
    }
    const rowBlocks = new Map<number, PositionedBlock[]>()
    for (const pb of placed.values()) {
        const row = rowBlocks.get(pb.y) ?? []
        row.push(pb)
        rowBlocks.set(pb.y, row)
    }
    runRowSeparation(rowBlocks, parentOfBlock, childrenOfBlock, placed)

    // Bottom-up parent recenter. The row-separation pass above shifts a
    // colliding block + its DESCENDANTS right by a delta — but it does
    // NOT touch the block's ANCESTORS. A child that gets pushed right by
    // a sibling collision therefore drifts out from under its parent
    // block, and the parent edge ends up diagonal / crossing the row above.
    // Bug-3 (Lau-style multi-row crossing) symptom.
    //
    // Fix: after row separation, walk each parent block bottom-up and
    // shift it horizontally so it sits centred over its children's
    // midpoint. That can in turn collide the parent with its OWN row
    // neighbours, so we re-run row separation and iterate to a fixed
    // point. Cap at a small number of rounds — the heuristic provably
    // converges for acyclic block graphs and a cap keeps a worst-case
    // pathological input from blowing through the frame budget.
    //
    // Roots are intentionally exempt: their order is fixed by
    // `reorderRootsByBarycenter` above and shifting them here would
    // undo that pass's work for marginal gain. Recentering children
    // alone is enough to straighten the typical parent-edge cross.
    for (let pass = 0; pass < 4; pass += 1) {
        if (!recenterParentsOverChildren(placed, childrenOfBlock, parentOfBlock, childrenOfPerson)) break
        runRowSeparation(rowBlocks, parentOfBlock, childrenOfBlock, placed)
    }

    // Compaction. Sequential root placement parks a disjoint family
    // cluster (a root subtree with no parent edges INTO the rest of the
    // tree) at the far-right edge, after the previous cluster's whole
    // subtree — leaving a wide gap when its only link is a child who
    // married into another cluster (the bridged couple hangs on the
    // spouse's side). Pull each root cluster as far LEFT as per-row
    // collisions allow so it sits close to its connection point. The
    // shift is rigid (whole cluster moves together) so parent-over-child
    // centering and within-cluster edges are preserved; only inter-
    // cluster gaps shrink, never below CLUSTER_GAP.
    compactRootClusters(rootBlocks, childrenOfBlock, placed)

    return materializeLayout(input, byId, placed)
}
