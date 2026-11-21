// Pass-2 reordering heuristics for `layoutTree`: a placement pass plus the
// barycenter root re-sort and the two-person couple swap that are driven
// by pass-1 positions.

import { layoutSubtree } from './subtree'
import { type Block, CLUSTER_GAP, NODE_W, type PositionedBlock, type TreeInput } from './types'

/**
 * Snapshot each positioned person's card-centre x from the current
 * placement so descendant walks are a cheap O(descendants) lookup.
 */
export function personCenterXMap(placed: Map<string, PositionedBlock>): Map<string, number> {
    const personCenterX = new Map<string, number>()
    for (const pb of placed.values()) {
        for (let i = 0; i < pb.members.length; i += 1) {
            const memberId = pb.members[i]
            if (memberId === undefined) continue
            const offset = pb.memberOffsets[i] ?? 0
            personCenterX.set(memberId, pb.x + offset + NODE_W / 2)
        }
    }
    return personCenterX
}

/**
 * Internal: run a single placement pass against the current root order and
 * each block's current `members` shape. Returns the freshly placed map so
 * the outer function can compare passes / iterate.
 */
export function runPlacement(
    rootBlocks: Block[],
    childrenOfBlock: Map<string, Block[]>,
    byId: Map<string, TreeInput['nodes'][number]>,
    bioParents: Map<string, Set<string>>,
): Map<string, PositionedBlock> {
    const placed = new Map<string, PositionedBlock>()
    const cursor = { x: 0 }
    for (let i = 0; i < rootBlocks.length; i += 1) {
        if (i > 0) cursor.x += CLUSTER_GAP
        const root = rootBlocks[i]
        if (root === undefined) continue
        layoutSubtree(root, childrenOfBlock, byId, bioParents, placed, cursor)
    }
    return placed
}

/**
 * Re-sort `rootBlocks` so each root sits above its descendants' centre of
 * mass (the "barycenter heuristic" from layered graph drawing). Returns
 * `true` when the order changed so the caller can decide whether to re-run
 * placement. The new order preserves the default ordering as a tie-breaker
 * — equal barycenters fall back to birth-date / id sort.
 *
 * Descendant collection walks `childrenOfPerson` (the raw parent_edges
 * graph) rather than `childrenOfBlock` (the block tree built from
 * `chooseParentBlock`). The block-tree walk misses the case where two
 * top-row mothers each have a child INSIDE the same multi-couple chain:
 *
 *   Top row: [mother_a (root), mother_b (root)]
 *   Middle row: chain block [son_a, anchor, son_b] where `anchor` is
 *               the shared spouse with son_a as ENDED partner + son_b
 *               as OPEN. `chooseParentBlock` walks the chain in order
 *               and picks the first parent-bearing member → son_a →
 *               mother_a's block. son_b's actual mother (mother_b)
 *               is invisible to the block tree.
 *
 * Walking parent_edges-of-persons instead exposes BOTH parental lineages
 * — mother_b's barycenter then equals son_b's x (her only descendant via
 * parent_edges), mother_a's equals son_a's. The pair sorts so each
 * mother ends up above her own child, eliminating the crossing parent
 * edge.
 */
export function reorderRootsByBarycenter(
    rootBlocks: Block[],
    placed: Map<string, PositionedBlock>,
    childrenOfPerson: Map<string, string[]>,
): boolean {
    if (rootBlocks.length < 2) return false
    const before = rootBlocks.map((b) => b.id).join('|')

    // Snapshot each positioned person's card-centre x once so the
    // descendant walk is a cheap O(descendants) per root.
    const personCenterX = personCenterXMap(placed)

    const barycenters = new Map<string, number>()
    for (const root of rootBlocks) {
        const sumCount = sumDescendantsByParentEdges(root, childrenOfPerson, personCenterX)
        // Roots with NO descendants (orphan leaves) keep their existing x as
        // the barycenter so they don't all collapse to 0 and reshuffle the
        // surviving roots.
        const final =
            sumCount.count === 0 ? (personCenterX.get(root.members[0] ?? '') ?? 0) : sumCount.sum / sumCount.count
        barycenters.set(root.id, final)
    }
    rootBlocks.sort((a, b) => {
        const ba = barycenters.get(a.id) ?? 0
        const bb = barycenters.get(b.id) ?? 0
        return ba - bb
    })
    return rootBlocks.map((b) => b.id).join('|') !== before
}

/**
 * BFS from each member of `root` through `childrenOfPerson` and accumulate
 * the descendant card-centre x values. Crucially this walks ALL
 * parent_edges, not just the canonical block-tree path — so a root whose
 * only child sits inside a multi-couple chain owned by ANOTHER root's
 * block tree still gets credit for that child in its barycenter.
 *
 * `seen` is shared within each root's BFS so a person reachable via
 * two paths from the same root (e.g. a grandchild reachable through
 * both parents who are siblings inside the same chain) is counted
 * ONCE per root walk. Double-counting across roots is fine and
 * actually wanted — when two top-row mothers share a multi-couple
 * chain in the next row, both should see the chain's grandchildren as
 * descendants because both share custody of the children's block.
 */
function sumDescendantsByParentEdges(
    root: Block,
    childrenOfPerson: Map<string, string[]>,
    personCenterX: Map<string, number>,
): { sum: number; count: number } {
    const seen = new Set<string>(root.members)
    const queue: string[] = [...root.members]
    let sum = 0
    let count = 0
    while (queue.length > 0) {
        const cur = queue.pop()
        if (cur === undefined) continue
        for (const k of childrenOfPerson.get(cur) ?? []) {
            if (seen.has(k)) continue
            seen.add(k)
            queue.push(k)
            const x = personCenterX.get(k)
            if (x !== undefined) {
                sum += x
                count += 1
            }
        }
    }
    return { sum, count }
}

/**
 * For each 2-person couple block whose members come from DIFFERENT parent
 * blocks placed on opposite sides of the row above, reverse the member
 * order so each spouse sits closer to their own parents. Mutates the
 * blocks in place. Returns `true` if any block was reversed so the caller
 * can re-run placement.
 */
export function swapTwoPersonCouplesByParentX(
    blocksByRow: Map<number, Block[]>,
    placed: Map<string, PositionedBlock>,
    byId: Map<string, TreeInput['nodes'][number]>,
    parentsOfPerson: Map<string, string[]>,
): boolean {
    let any = false
    for (const list of blocksByRow.values()) {
        for (const block of list) {
            if (block.members.length !== 2) continue
            const aId = block.members[0]
            const bId = block.members[1]
            if (aId === undefined || bId === undefined) continue
            const aParentX = canonicalParentBlockX(aId, parentsOfPerson, placed, byId)
            const bParentX = canonicalParentBlockX(bId, parentsOfPerson, placed, byId)
            // Only act when BOTH spouses have a parent block placed AND
            // those parents differ in x. Either missing → no information to
            // act on, keep the default id-sorted order.
            if (aParentX === null || bParentX === null) continue
            if (Math.abs(aParentX - bParentX) < 1) continue
            // If `b`'s parent is to the LEFT of `a`'s parent, the right-
            // hand member (`b`) should be on the LEFT of the couple →
            // swap. Otherwise leave it.
            if (bParentX < aParentX) {
                block.members = [bId, aId]
                any = true
            }
        }
    }
    return any
}

/**
 * Centre x of the parent BLOCK containing the canonical (smallest-id)
 * parent of `personId`. `null` when the person has no parents in the
 * data or the parent block hasn't been placed yet.
 */
function canonicalParentBlockX(
    personId: string,
    parentsOfPerson: Map<string, string[]>,
    placed: Map<string, PositionedBlock>,
    byId: Map<string, TreeInput['nodes'][number]>,
): number | null {
    const parents = parentsOfPerson.get(personId) ?? []
    if (parents.length === 0) return null
    const sortedParents = [...parents].sort()
    for (const pid of sortedParents) {
        if (!byId.has(pid)) continue
        for (const pb of placed.values()) {
            if (pb.members.includes(pid)) {
                return pb.x + pb.pixelWidth / 2
            }
        }
    }
    return null
}
