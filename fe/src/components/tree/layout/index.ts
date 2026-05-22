// SVG layout for the family tree. v2: block-based layout where each
// "couple" (two same-row partners) is the placement unit. Children sit
// centered under their parent block; sibling groups of different parent
// blocks are separated by a wider CLUSTER_GAP so the visual grouping is
// unambiguous. Top-row blocks are laid out left-to-right by birth date
// in stable id order. Generation rank is computed independently from
// canonical-parent edges so older parentless ancestors still land above
// younger rows.

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
 * Strategy (v2):
 *   1. Build the full child-of-person adjacency from parent_edges. Compute
 *      a generation index per person (bottom-up over that adjacency) and
 *      promote eldest orphans by birth-year gap.
 *   2. Build per-row blocks: each pair of same-row partners becomes a couple
 *      block; singletons get their own block.
 *   3. Choose a canonical parent block for each non-top block (the block
 *      containing the smallest-id parent of the block's anchor member).
 *      Top-row blocks have no parent block.
 *   4. Place top-row blocks left-to-right with CLUSTER_GAP between them.
 *      Recursively lay out each top block's subtree: children placed
 *      left-to-right by birth date, then the parent block is centered over
 *      the children cluster.
 *   5. Sweep each row for inter-cluster collisions and enforce COL_GAP
 *      within a cluster, CLUSTER_GAP between clusters belonging to
 *      different parent blocks.
 *   6. Materialize positioned persons by reading each block's x and
 *      distributing it over the 1 or 2 members. Emit parent + partner
 *      edges from those coordinates.
 */
export function layoutTree(input: TreeInput): LayoutResult {
    const byId = new Map(input.nodes.map((n) => [n.id, n]))

    // Full parent adjacency for the generation-rank pass (and for the
    // post-layout parent-edge render). EdgePair: `a` = child, `b` = parent.
    const childrenOfPerson = new Map<string, string[]>()
    for (const e of input.parent_edges) {
        if (!byId.has(e.a) || !byId.has(e.b)) continue
        const list = childrenOfPerson.get(e.b) ?? []
        list.push(e.a)
        childrenOfPerson.set(e.b, list)
    }

    const baseGeneration = computeGenerations(
        input.nodes.map((n) => n.id),
        childrenOfPerson,
    )
    const generation = promoteEldestOrphans(input.nodes, baseGeneration, childrenOfPerson)
    let topGen = 0
    for (const g of generation.values()) if (g > topGen) topGen = g

    // Partner adjacency. Both directions get stored so the buildBlocks pass
    // is symmetric regardless of which member's id appears first in the
    // edge. Filter to known persons.
    const partnerOf = new Map<string, Set<string>>()
    for (const e of input.partner_edges) {
        if (!byId.has(e.a) || !byId.has(e.b)) continue
        const sa = partnerOf.get(e.a) ?? new Set<string>()
        sa.add(e.b)
        partnerOf.set(e.a, sa)
        const sb = partnerOf.get(e.b) ?? new Set<string>()
        sb.add(e.a)
        partnerOf.set(e.b, sb)
    }

    const blocksByRow = buildBlocks(
        input.nodes.map((n) => n.id),
        generation,
        partnerOf,
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
        layoutSubtree(root, childrenOfBlock, byId, placed, cursor)
    }

    // Per-row separation pass. The recursive layout guarantees no overlap
    // *within* a subtree, but two subtrees on the same row may collide if a
    // descendant cluster is wider than the available slot. Walk each row
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
            const prevWidth = prev.width * NODE_W + Math.max(0, prev.width - 1) * COL_GAP
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

    // Materialize positioned persons. For couples, the LEFT member sits at
    // the block's x, the RIGHT member at x + NODE_W + COL_GAP.
    const positioned = new Map<string, Positioned>()
    for (const pb of placed.values()) {
        for (let i = 0; i < pb.members.length; i += 1) {
            const memberId = pb.members[i]
            if (memberId === undefined) continue
            const n = byId.get(memberId)
            if (n === undefined) continue
            const memberX = pb.x + i * (NODE_W + COL_GAP)
            positioned.set(memberId, {
                id: memberId,
                given_name: n.given_name,
                family_name: n.family_name,
                birth_date: n.birth_date ?? null,
                death_date: n.death_date ?? null,
                linked_user_id: n.linked_user_id ?? null,
                x: memberX,
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
