// Recursive subtree placement for layout v2. Given a block tree (each
// non-root block points at one parent block) this walks each top block
// bottom-up, places children left-to-right by birth date, then centers
// each parent on the children's mid-point. Per-row sweeps that fix up
// inter-cluster gaps live in the main `index.ts` orchestrator — this
// module only does the depth-first subtree math.

import { blockSortKey, compareBlockKeys } from './blocks'
import { type BackendNode, type Block, COL_GAP, NODE_W, type PositionedBlock } from './types'

/**
 * Recursive subtree placement. Returns the placed x-extent (`[xL, xR]`) of
 * the subtree rooted at `block`.
 *
 *   1. Lay out all of `block`'s children first, left-to-right with COL_GAP
 *      inside the cluster.
 *   2. Center `block` on the children's mid-point. If the resulting block
 *      reaches left past the cluster's leftmost child, shift the entire
 *      subtree right so the block's left edge sits at the cluster's
 *      original leftmost x. This preserves the per-row "no overlap"
 *      invariant by guaranteeing the block never reaches into the
 *      previous sibling's space.
 *   3. If `block` has no children, it gets placed at the leftmost free x
 *      passed in by the caller via the `cursor` ref.
 *
 * Each block contributes `width * NODE_W + (width - 1) * COL_GAP` columns.
 */
export function layoutSubtree(
    block: Block,
    childrenOfBlock: Map<string, Block[]>,
    nodeById: Map<string, BackendNode>,
    placed: Map<string, PositionedBlock>,
    cursor: { x: number },
): { xL: number; xR: number } {
    const blockWidth = block.width * NODE_W + Math.max(0, block.width - 1) * COL_GAP
    const children = childrenOfBlock.get(block.id) ?? []
    if (children.length === 0) {
        const xL = cursor.x
        const xR = xL + blockWidth
        placed.set(block.id, { ...block, x: xL })
        cursor.x = xR
        return { xL, xR }
    }

    // Sort children left-to-right by birth date (oldest first) then id.
    const sortedChildren = [...children].sort((a, b) =>
        compareBlockKeys(blockSortKey(a, nodeById), blockSortKey(b, nodeById)),
    )

    let firstL = Number.POSITIVE_INFINITY
    let lastR = Number.NEGATIVE_INFINITY
    for (let i = 0; i < sortedChildren.length; i += 1) {
        if (i > 0) cursor.x += COL_GAP
        const child = sortedChildren[i]
        if (child === undefined) continue
        const { xL, xR } = layoutSubtree(child, childrenOfBlock, nodeById, placed, cursor)
        if (xL < firstL) firstL = xL
        if (xR > lastR) lastR = xR
    }
    if (!Number.isFinite(firstL)) {
        const xL = cursor.x
        placed.set(block.id, { ...block, x: xL })
        cursor.x = xL + blockWidth
        return { xL, xR: cursor.x }
    }

    const childrenMid = (firstL + lastR) / 2
    let blockL = childrenMid - blockWidth / 2
    if (blockL < firstL) {
        const delta = firstL - blockL
        for (const child of sortedChildren) {
            shiftSubtree(child, childrenOfBlock, placed, delta)
        }
        blockL = firstL
        cursor.x = lastR + delta
    }
    const blockR = blockL + blockWidth
    // Block never *recedes* past lastR — if children are wider than the
    // block, the cursor is already past blockR.
    if (blockR > cursor.x) cursor.x = blockR
    placed.set(block.id, { ...block, x: blockL })
    return { xL: Math.min(blockL, firstL), xR: Math.max(blockR, lastR) }
}

/**
 * Shift every already-placed block in this subtree right by `delta`. Used
 * when centering a parent over its children requires moving the children
 * cluster to make room.
 */
export function shiftSubtree(
    block: Block,
    childrenOfBlock: Map<string, Block[]>,
    placed: Map<string, PositionedBlock>,
    delta: number,
): void {
    const p = placed.get(block.id)
    if (p !== undefined) placed.set(block.id, { ...p, x: p.x + delta })
    const kids = childrenOfBlock.get(block.id) ?? []
    for (const k of kids) shiftSubtree(k, childrenOfBlock, placed, delta)
}
