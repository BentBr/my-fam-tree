// Post-placement straightening for `layoutTree`: per-row collision
// separation and the bottom-up parent-over-children recenter pass.

import { personCenterXMap } from './reorder'
import { centerLeftOver, shiftSubtree } from './subtree'
import { type Block, CLUSTER_GAP, COL_GAP, NODE_W, type PositionedBlock } from './types'

/**
 * Bottom-up parent-over-children recentering. For each parent block,
 * recompute the children's midpoint from their CURRENT positions (post
 * row-separation) and shift the block so it sits centred above that
 * midpoint.
 *
 *   - NON-root blocks: midpoint comes from the direct block-tree
 *     children (`childrenOfBlock`). Matches the initial-placement
 *     centering rule in `layoutSubtree`.
 *   - ROOT blocks: midpoint comes from the direct PARENT-EDGE children
 *     (`childrenOfPerson` from each block member). The block tree
 *     misses one parent of a multi-couple chain (the "second mother"
 *     case — a chain [son_a, anchor, son_b] is owned by son_a's
 *     mother only, son_b's mother sees no block-tree descendants), so
 *     a root with no block-tree descendant but a real parent_edge
 *     descendant ends up marooned at the far edge of the row. Using
 *     parent-edge children pulls each parent over to sit above their
 *     own child, and similarly pulls a far-off root cluster (e.g. an
 *     in-laws couple whose daughter married into the main tree) close
 *     to its real connection point instead of being banished to an
 *     arbitrary `cursor + CLUSTER_GAP` slot.
 *
 * Returns `true` when at least one block was shifted by > 0.5 px so the
 * caller can decide whether to re-run row separation + iterate. The
 * sub-px deadband keeps a successful recenter from being re-detected as
 * "still drifting" forever due to floating-point round-trips.
 *
 * Only the parent block itself moves — NOT its descendants. The next
 * row-separation pass picks up any collisions the parent shift created
 * in its own row.
 */
export function recenterParentsOverChildren(
    placed: Map<string, PositionedBlock>,
    childrenOfBlock: Map<string, Block[]>,
    parentOfBlock: Map<string, string | null>,
    childrenOfPerson: Map<string, string[]>,
): boolean {
    // Snapshot person card-centres once for the root-pass parent-edge
    // descendant lookup; the non-root pass uses the block extents in
    // `placed` directly.
    const personCenterX = personCenterXMap(placed)

    // Process bottom rows first (higher y) so each parent's recompute sees
    // the most-recent positions of its children.
    const candidates: PositionedBlock[] = [...placed.values()]
    candidates.sort((a, b) => b.y - a.y)

    let any = false
    for (const pb of candidates) {
        const isRoot = parentOfBlock.get(pb.id) === undefined
        const extent = isRoot
            ? rootChildExtentByParentEdges(pb, childrenOfPerson, personCenterX, childrenOfBlock)
            : blockChildExtent(childrenOfBlock.get(pb.id) ?? [], placed)
        if (extent === null) continue
        // Shared with `layoutSubtree`'s initial-placement centering —
        // the math for "place a block of width W so its centre sits at
        // (L + R) / 2" lives in `centerLeftOver` so a future tweak
        // (different centering rule, weighted midpoint, …) updates both.
        const targetL = centerLeftOver(extent.L, extent.R, pb.pixelWidth)
        const delta = targetL - pb.x
        if (Math.abs(delta) > 0.5) {
            placed.set(pb.id, { ...pb, x: pb.x + delta })
            any = true
        }
    }
    return any
}

/**
 * Direct-children L/R extent for a non-root block. Returns `null` when
 * no children are placed (childless block — caller skips it).
 */
function blockChildExtent(kids: Block[], placed: Map<string, PositionedBlock>): { L: number; R: number } | null {
    let firstL = Number.POSITIVE_INFINITY
    let lastR = Number.NEGATIVE_INFINITY
    for (const k of kids) {
        const cp = placed.get(k.id)
        if (cp === undefined) continue
        if (cp.x < firstL) firstL = cp.x
        const r = cp.x + cp.pixelWidth
        if (r > lastR) lastR = r
    }
    return Number.isFinite(firstL) ? { L: firstL, R: lastR } : null
}

/**
 * Parent-edge direct-children L/R extent for a root block, used to
 * recentre the root above its IMMEDIATE descendants (direct children
 * only — not grandchildren). Centre positions come from the children's
 * card centres (± NODE_W/2).
 *
 * Children are split into:
 *   - OWN: the child sits in a block that actually hangs under this root
 *     in the block tree (a member of one of `childrenOfBlock[root]`).
 *   - BRIDGED: the child married out — their block hangs under the
 *     spouse's (more-connected) family elsewhere, so the child is
 *     positioned far away.
 *
 * If the root has any OWN children we centre over THOSE only — so a root
 * with some kids in its own subtree and one kid married out doesn't get
 * dragged toward the married-out kid (the Quanten case: Quirin+Sari stay
 * above Kira+Juna, not pulled left toward the bridged-away Marit). If the
 * root has NO own children (its only child lives inside another root's
 * chain — the "second mother" / Herta case) we fall back to ALL
 * parent-edge children so the root still sits above its sole child.
 *
 * Returns `null` when the root has no parent-edge children at all.
 */
function rootChildExtentByParentEdges(
    root: PositionedBlock,
    childrenOfPerson: Map<string, string[]>,
    personCenterX: Map<string, number>,
    childrenOfBlock: Map<string, Block[]>,
): { L: number; R: number } | null {
    // Members of the blocks that actually hang under this root.
    const ownMembers = new Set<string>()
    for (const childBlock of childrenOfBlock.get(root.id) ?? []) {
        for (const m of childBlock.members) ownMembers.add(m)
    }
    const edgeKids: string[] = []
    for (const memberId of root.members) {
        for (const kid of childrenOfPerson.get(memberId) ?? []) edgeKids.push(kid)
    }
    const ownKids = edgeKids.filter((k) => ownMembers.has(k))
    const target = ownKids.length > 0 ? ownKids : edgeKids

    let firstL = Number.POSITIVE_INFINITY
    let lastR = Number.NEGATIVE_INFINITY
    for (const kid of target) {
        const cx = personCenterX.get(kid)
        if (cx === undefined) continue
        const l = cx - NODE_W / 2
        const r = cx + NODE_W / 2
        if (l < firstL) firstL = l
        if (r > lastR) lastR = r
    }
    return Number.isFinite(firstL) ? { L: firstL, R: lastR } : null
}

/**
 * Run a single per-row separation pass against the current placement.
 * Walks each row left-to-right and shifts a block (plus its descendants)
 * right by whatever delta is needed to satisfy the COL_GAP /
 * CLUSTER_GAP floor against its left neighbour.
 *
 * Extracted from the inline loop in `layoutTree` so the recenter +
 * row-sep iteration can call it more than once.
 */
export function runRowSeparation(
    rowBlocks: Map<number, PositionedBlock[]>,
    parentOfBlock: Map<string, string | null>,
    childrenOfBlock: Map<string, Block[]>,
    placed: Map<string, PositionedBlock>,
): void {
    for (const row of rowBlocks.values()) {
        // Re-read each block's CURRENT x (a recenter pass may have shifted
        // a parent in this row since the last sort).
        const sorted = [...row]
            .map((b) => placed.get(b.id))
            .filter((b): b is PositionedBlock => b !== undefined)
            .sort((a, b) => a.x - b.x)
        for (let i = 1; i < sorted.length; i += 1) {
            const prev = sorted[i - 1]
            const curr = sorted[i]
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
                // Keep the local snapshot in sync with the canonical map
                // for subsequent comparisons in this pass.
                const refreshed = placed.get(curr.id)
                if (refreshed !== undefined) sorted[i] = refreshed
            }
        }
    }
}
