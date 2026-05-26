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
import { layoutSubtree, shiftSubtree } from './subtree'
import {
    type Block,
    CLUSTER_GAP,
    COL_GAP,
    type LayoutResult,
    NODE_H,
    NODE_W,
    type ParentEdge,
    type PartnerEdge,
    type Positioned,
    type PositionedBlock,
    ROW_GAP,
    type TreeInput,
} from './types'

export {
    type BackendEdge,
    type BackendNode,
    type BackendPartnerEdge,
    CLUSTER_GAP,
    COL_GAP,
    type LayoutResult,
    NODE_H,
    NODE_W,
    type ParentEdge,
    type PartnerEdge,
    type Positioned,
    ROW_GAP,
    type TreeInput,
} from './types'

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

    const placed = new Map<string, PositionedBlock>()
    const cursor = { x: 0 }
    for (let i = 0; i < rootBlocks.length; i += 1) {
        if (i > 0) cursor.x += CLUSTER_GAP
        const root = rootBlocks[i]
        if (root === undefined) continue
        layoutSubtree(root, childrenOfBlock, byId, bioParents, placed, cursor)
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
    for (const row of rowBlocks.values()) {
        row.sort((a, b) => a.x - b.x)
        for (let i = 1; i < row.length; i += 1) {
            const prev = row[i - 1]
            const curr = row[i]
            if (prev === undefined || curr === undefined) continue
            const prevWidth = prev.pixelWidth
            const sameParent =
                parentOfBlock.get(prev.id) !== undefined &&
                parentOfBlock.get(prev.id) === parentOfBlock.get(curr.id) &&
                parentOfBlock.get(prev.id) !== null
            const gap = sameParent ? COL_GAP : CLUSTER_GAP
            const floor = prev.x + prevWidth + gap
            if (curr.x < floor) {
                const delta = floor - curr.x
                shiftSubtree(curr, childrenOfBlock, placed, delta)
            }
        }
    }

    // Materialize positioned persons. Each block carries its own per-member
    // x offset array — for default-spaced blocks that's
    // `i * (NODE_W + COL_GAP)`; for widened multi-couple blocks the
    // offsets grow to align each couple-midpoint with its sub-cluster.
    const positioned = new Map<string, Positioned>()
    for (const pb of placed.values()) {
        for (let i = 0; i < pb.members.length; i += 1) {
            const memberId = pb.members[i]
            if (memberId === undefined) continue
            const n = byId.get(memberId)
            if (n === undefined) continue
            const offset = pb.memberOffsets[i] ?? i * (NODE_W + COL_GAP)
            positioned.set(memberId, {
                id: memberId,
                given_name: n.given_name,
                family_name: n.family_name,
                birth_date: n.birth_date ?? null,
                death_date: n.death_date ?? null,
                linked_user_id: n.linked_user_id ?? null,
                is_favourite_for_me: n.is_favourite_for_me ?? false,
                x: pb.x + offset,
                y: pb.y,
            })
        }
    }

    // Shift so minX == 0; collect bounds.
    let minX = Number.POSITIVE_INFINITY
    let maxX = Number.NEGATIVE_INFINITY
    let maxY = 0
    for (const p of positioned.values()) {
        if (p.x < minX) minX = p.x
        if (p.x > maxX) maxX = p.x
        if (p.y > maxY) maxY = p.y
    }
    if (!Number.isFinite(minX)) minX = 0
    if (!Number.isFinite(maxX)) maxX = 0
    for (const p of positioned.values()) {
        p.x -= minX
    }

    const parentEdges: ParentEdge[] = input.parent_edges.flatMap((e) => {
        // `a` is the child, `b` is the parent — matches `EdgePair` on the wire.
        const c = positioned.get(e.a)
        const p = positioned.get(e.b)
        if (c === undefined || p === undefined) return []
        return [
            {
                childId: e.a,
                parentId: e.b,
                childX: c.x + NODE_W / 2,
                childY: c.y,
                parentX: p.x + NODE_W / 2,
                parentY: p.y + NODE_H,
            },
        ]
    })

    const partnerEdges: PartnerEdge[] = input.partner_edges.flatMap((e) => {
        const a = positioned.get(e.a)
        const b = positioned.get(e.b)
        if (a === undefined || b === undefined) return []
        const leftIsA = a.x <= b.x
        const left = leftIsA ? a : b
        const right = leftIsA ? b : a
        return [
            {
                aId: leftIsA ? e.a : e.b,
                bId: leftIsA ? e.b : e.a,
                ax: left.x + NODE_W,
                ay: left.y + NODE_H / 2,
                bx: right.x,
                by: right.y + NODE_H / 2,
            },
        ]
    })

    return {
        nodes: Array.from(positioned.values()),
        parentEdges,
        partnerEdges,
        width: maxX - minX + NODE_W,
        height: maxY + NODE_H,
    }
}
