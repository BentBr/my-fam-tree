// Recursive subtree placement for layout v2/v3. Given a block tree (each
// non-root block points at one parent block) this walks each top block
// bottom-up, places children left-to-right by birth date, then centers
// each parent on the children's mid-point. Per-row sweeps that fix up
// inter-cluster gaps live in the main `index.ts` orchestrator — this
// module only does the depth-first subtree math.
//
// v3.1 multi-couple support: when a block has more than one internal couple
// (e.g. "Brigitte, Klaus, Anna" with Klaus shared between an ended and an
// open partnership), the children of each couple form their own sub-cluster.
// Each sub-cluster centres under its bio-couple midpoint inside the block;
// if the natural inter-couple spacing isn't enough to fit both sub-clusters
// without overlap, the block's internal gaps widen to compensate.

import { blockSortKey, compareBlockKeys } from './blocks'
import {
    type BackendNode,
    type Block,
    type BlockCouple,
    CLUSTER_GAP,
    COL_GAP,
    NODE_W,
    type PositionedBlock,
} from './types'

/** Per-couple grouping of a block's children, used by the multi-couple path. */
interface CouplePlan {
    couple: BlockCouple
    children: Block[]
}

/** A computed sub-cluster: the placed children of one internal couple. */
interface SubCluster {
    couple: BlockCouple
    childIds: string[]
    width: number
}

function defaultMemberOffsets(count: number): number[] {
    const out: number[] = []
    for (let i = 0; i < count; i += 1) out.push(i * (NODE_W + COL_GAP))
    return out
}

function blockPixelWidth(memberOffsets: number[]): number {
    if (memberOffsets.length === 0) return 0
    const last = memberOffsets[memberOffsets.length - 1] ?? 0
    return last + NODE_W
}

/**
 * Group a block's children by which internal couple produced them. A child
 * is matched to a couple when BOTH of the couple's members appear in the
 * child's biological parent set. Children that don't match any internal
 * couple — e.g. step-only links, or single-parent children inside a
 * multi-couple block — fall back to the rightmost couple so they sit
 * visually under the current partnership.
 */
function groupChildrenByCouple(block: Block, children: Block[], bioParents: Map<string, Set<string>>): CouplePlan[] {
    if (block.couples.length === 0) return []
    const plans: CouplePlan[] = block.couples.map((c) => ({ couple: c, children: [] }))
    const rightmost = plans[plans.length - 1]
    for (const child of children) {
        const anchor = child.members[0]
        if (anchor === undefined) continue
        const parents = bioParents.get(anchor) ?? new Set<string>()
        let matched = false
        for (const plan of plans) {
            const lId = block.members[plan.couple.leftIdx]
            const rId = block.members[plan.couple.rightIdx]
            if (lId === undefined || rId === undefined) continue
            if (parents.has(lId) && parents.has(rId)) {
                plan.children.push(child)
                matched = true
                break
            }
        }
        if (!matched && rightmost !== undefined) rightmost.children.push(child)
    }
    return plans
}

/**
 * Recursive subtree placement. Returns the placed x-extent (`[xL, xR]`) of
 * the subtree rooted at `block`.
 *
 * Singletons + 2-member couples behave exactly as v2:
 *   1. Lay out all children first, left-to-right with COL_GAP inside the
 *      cluster.
 *   2. Center `block` on the children's mid-point. If the resulting block
 *      reaches left past the cluster's leftmost child, shift the entire
 *      subtree right so the block's left edge sits at the cluster's
 *      original leftmost x.
 *   3. If `block` has no children, place at the leftmost free x passed in
 *      by the caller via the `cursor` ref.
 *
 * 3-or-more member blocks (multi-couple) instead place each sub-cluster
 * under its bio-couple midpoint and widen the block's internal gaps to
 * absorb sub-cluster overlap.
 */
export function layoutSubtree(
    block: Block,
    childrenOfBlock: Map<string, Block[]>,
    nodeById: Map<string, BackendNode>,
    bioParents: Map<string, Set<string>>,
    placed: Map<string, PositionedBlock>,
    cursor: { x: number },
): { xL: number; xR: number } {
    const defaultOffsets = defaultMemberOffsets(block.members.length)
    const defaultWidth = blockPixelWidth(defaultOffsets)
    const children = childrenOfBlock.get(block.id) ?? []
    if (children.length === 0) {
        const xL = cursor.x
        const xR = xL + defaultWidth
        placed.set(block.id, { ...block, x: xL, memberOffsets: defaultOffsets, pixelWidth: defaultWidth })
        cursor.x = xR
        return { xL, xR }
    }

    // Multi-couple path: ≥ 2 internal couples means children grouping +
    // per-couple sub-clusters with widened internal gaps.
    if (block.couples.length >= 2) {
        return layoutMultiCouple(block, children, childrenOfBlock, nodeById, bioParents, placed, cursor)
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
        const { xL, xR } = layoutSubtree(child, childrenOfBlock, nodeById, bioParents, placed, cursor)
        if (xL < firstL) firstL = xL
        if (xR > lastR) lastR = xR
    }
    if (!Number.isFinite(firstL)) {
        const xL = cursor.x
        placed.set(block.id, { ...block, x: xL, memberOffsets: defaultOffsets, pixelWidth: defaultWidth })
        cursor.x = xL + defaultWidth
        return { xL, xR: cursor.x }
    }

    const childrenMid = (firstL + lastR) / 2
    let blockL = childrenMid - defaultWidth / 2
    if (blockL < firstL) {
        const delta = firstL - blockL
        for (const child of sortedChildren) {
            shiftSubtree(child, childrenOfBlock, placed, delta)
        }
        blockL = firstL
        cursor.x = lastR + delta
    }
    const blockR = blockL + defaultWidth
    // Block never *recedes* past lastR — if children are wider than the
    // block, the cursor is already past blockR.
    if (blockR > cursor.x) cursor.x = blockR
    placed.set(block.id, { ...block, x: blockL, memberOffsets: defaultOffsets, pixelWidth: defaultWidth })
    return { xL: Math.min(blockL, firstL), xR: Math.max(blockR, lastR) }
}

/**
 * Multi-couple block layout. The block's members stay in the threaded order
 * built by `blocks.ts`; what we compute here is each member's x offset
 * inside the block so every bio-couple sits exactly above the midpoint of
 * its own children sub-cluster.
 *
 * Algorithm:
 *   1. Group children by couple (`groupChildrenByCouple`). Sort each group
 *      left-to-right by birth date.
 *   2. Lay out each sub-cluster in isolation (cursor starts at 0) so we
 *      learn each sub-cluster's width.
 *   3. Pack sub-clusters left-to-right with CLUSTER_GAP between adjacent
 *      ones; empty clusters (couple with no children) collapse to a
 *      single-node-wide placeholder so the block still grows enough to fit
 *      two cards plus their gap.
 *   4. Derive each couple's target midpoint from its sub-cluster midpoint
 *      and convert into per-member offsets. Couples share an anchor member
 *      (the previous couple's right is the next couple's left), so we
 *      solve sequentially:
 *        first couple → symmetric around its midpoint with default
 *                        NODE_W + COL_GAP gap.
 *        subsequent   → left is the previous couple's right (already set);
 *                        right is whatever the equation requires, floored
 *                        at left + NODE_W + COL_GAP.
 *   5. Normalise to non-negative offsets, place each sub-cluster under its
 *      couple midpoint, set the block at `cursor.x`.
 */
function layoutMultiCouple(
    block: Block,
    children: Block[],
    childrenOfBlock: Map<string, Block[]>,
    nodeById: Map<string, BackendNode>,
    bioParents: Map<string, Set<string>>,
    placed: Map<string, PositionedBlock>,
    cursor: { x: number },
): { xL: number; xR: number } {
    const plans = groupChildrenByCouple(block, children, bioParents)

    // Lay out each sub-cluster in isolation so we know each cluster width.
    // After this pass each child in a non-empty cluster sits at xL inside
    // [0, cluster.width]; we shift them globally once the block's final
    // position is known.
    interface PlacedSub {
        plan: CouplePlan
        cluster: SubCluster
    }
    const subs: PlacedSub[] = []
    for (const plan of plans) {
        if (plan.children.length === 0) {
            subs.push({ plan, cluster: { couple: plan.couple, childIds: [], width: 0 } })
            continue
        }
        const sorted = [...plan.children].sort((a, b) =>
            compareBlockKeys(blockSortKey(a, nodeById), blockSortKey(b, nodeById)),
        )
        const subCursor = { x: 0 }
        let firstL = Number.POSITIVE_INFINITY
        let lastR = Number.NEGATIVE_INFINITY
        for (let i = 0; i < sorted.length; i += 1) {
            if (i > 0) subCursor.x += COL_GAP
            const child = sorted[i]
            if (child === undefined) continue
            const { xL, xR } = layoutSubtree(child, childrenOfBlock, nodeById, bioParents, placed, subCursor)
            if (xL < firstL) firstL = xL
            if (xR > lastR) lastR = xR
        }
        if (!Number.isFinite(firstL)) {
            subs.push({ plan, cluster: { couple: plan.couple, childIds: sorted.map((c) => c.id), width: 0 } })
            continue
        }
        // Normalise the sub-cluster to start at 0 so the later global shift
        // is a single add per child.
        const width = lastR - firstL
        for (const c of sorted) shiftSubtree(c, childrenOfBlock, placed, -firstL)
        subs.push({ plan, cluster: { couple: plan.couple, childIds: sorted.map((c) => c.id), width } })
    }

    // Compute desired couple midpoints by packing sub-clusters left-to-
    // right. Empty sub-clusters get a NODE_W placeholder so two adjacent
    // childless couples still leave room for the centre member's card.
    const EMPTY_CLUSTER_WIDTH = NODE_W
    const midpoints: number[] = []
    let runningX = 0
    for (let i = 0; i < subs.length; i += 1) {
        const sub = subs[i]
        if (sub === undefined) continue
        const w = sub.cluster.width === 0 ? EMPTY_CLUSTER_WIDTH : sub.cluster.width
        midpoints.push(runningX + w / 2)
        runningX += w
        if (i < subs.length - 1) runningX += CLUSTER_GAP
    }

    // Convert couple midpoints into per-member x offsets. Each couple
    // contributes:
    //   offset(left) + offset(right) + NODE_W == 2 * couple_mid
    // Couples share members (anchor), so we solve sequentially:
    //   first couple   → symmetric around its midpoint with the default
    //                    NODE_W + COL_GAP gap.
    //   subsequent     → left is the previous couple's right (already set);
    //                    right is whatever the equation requires, floored
    //                    at left + NODE_W + COL_GAP so adjacent cards never
    //                    overlap.
    // v3.2 revision: keep members at fixed `NODE_W + COL_GAP` spacing
    // regardless of children-cluster width. Earlier passes tried to align
    // each member directly above its couple's children midpoint, but when
    // one couple has many more children than the other the math stretches
    // the SHARED member's adjacent partner far to the right (Klaus + Anna
    // ended up ~720 px apart because Klaus + Anna had 3 bio kids while
    // Klaus + Brigitte had 1). Adjacency between partners wins; the
    // children sub-clusters re-center under whatever midpoint the fixed
    // member positions produce (Felix may sit slightly off-center under
    // Klaus + Brigitte, but the visual relationship stays readable).
    const memberOffsets: number[] = new Array<number>(block.members.length).fill(0)
    for (let i = 0; i < memberOffsets.length; i += 1) {
        memberOffsets[i] = i * (NODE_W + COL_GAP)
    }
    // Recompute each couple's midpoint from the fixed member positions so
    // the sub-cluster shift below targets the new geometry.
    for (let ci = 0; ci < block.couples.length; ci += 1) {
        const couple = block.couples[ci]
        if (couple === undefined) continue
        const offL = memberOffsets[couple.leftIdx] ?? 0
        const offR = memberOffsets[couple.rightIdx] ?? 0
        midpoints[ci] = (offL + offR + NODE_W) / 2
    }

    // Normalise to non-negative offsets so the block's leftmost member
    // sits at 0. Apply the same shift to the sub-cluster midpoints so the
    // final placement stays consistent.
    let minOffset = Number.POSITIVE_INFINITY
    for (const o of memberOffsets) if (o < minOffset) minOffset = o
    if (!Number.isFinite(minOffset)) minOffset = 0
    for (let i = 0; i < memberOffsets.length; i += 1) {
        memberOffsets[i] = (memberOffsets[i] ?? 0) - minOffset
    }
    for (let i = 0; i < midpoints.length; i += 1) {
        midpoints[i] = (midpoints[i] ?? 0) - minOffset
    }

    // Place the block at cursor.x and shift each sub-cluster to align with
    // its couple midpoint. `xR` tracks the rightmost extent including any
    // children that overshoot the block. A `runningRight` cursor guards
    // against sub-cluster overlap: when one couple's children are much
    // wider than the inter-couple-midpoint spacing the centered placement
    // would land behind the previous cluster, so we push it right and
    // accept a slightly off-center sub-cluster (visible drift, not
    // collision — exactly what we want).
    const blockL = cursor.x
    const pixelWidth = blockPixelWidth(memberOffsets)
    placed.set(block.id, { ...block, x: blockL, memberOffsets, pixelWidth })
    let xL = blockL
    let xR = blockL + pixelWidth
    let runningRight = Number.NEGATIVE_INFINITY
    for (let i = 0; i < subs.length; i += 1) {
        const sub = subs[i]
        if (sub === undefined) continue
        if (sub.cluster.width === 0) continue
        const mid = midpoints[i] ?? 0
        const desiredLeft = blockL + mid - sub.cluster.width / 2
        // Floor the next sub-cluster at the previous one's right + COL_GAP
        // (not CLUSTER_GAP) so single-child clusters still settle exactly
        // on their couple midpoints — the wider CLUSTER_GAP would force a
        // ~24 px drift even when there's no overlap pressure.
        const subLeftAbsolute =
            runningRight === Number.NEGATIVE_INFINITY ? desiredLeft : Math.max(desiredLeft, runningRight + COL_GAP)
        for (const childId of sub.cluster.childIds) {
            const child = sub.plan.children.find((c) => c.id === childId)
            if (child === undefined) continue
            shiftSubtree(child, childrenOfBlock, placed, subLeftAbsolute)
        }
        if (subLeftAbsolute < xL) xL = subLeftAbsolute
        const subR = subLeftAbsolute + sub.cluster.width
        if (subR > xR) xR = subR
        runningRight = subR
    }
    cursor.x = xR
    return { xL, xR }
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
