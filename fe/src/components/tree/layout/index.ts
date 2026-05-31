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
    if (reorderRootsByBarycenter(rootBlocks, placed, childrenOfBlock)) changed = true
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
                photo_url: n.photo_url ?? null,
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

    // Snapshot the positioned values once so the adjacency probe below
    // doesn't re-allocate per partnership. Cheap O(N·P) overall — at our
    // family sizes this is fine.
    const positionedList = Array.from(positioned.values())
    const partnerEdges: PartnerEdge[] = input.partner_edges.flatMap((e) => {
        const a = positioned.get(e.a)
        const b = positioned.get(e.b)
        if (a === undefined || b === undefined) return []
        const leftIsA = a.x <= b.x
        const left = leftIsA ? a : b
        const right = leftIsA ? b : a
        // `kind` / `ended` ride the wire payload (see `BackendPartnerEdge`)
        // and surface to TreeEdge so it can pick glyph + colour. Fixtures
        // that ship a bare `{a, b}` pair land here as `kind: null` +
        // `ended: false`, defaulting to "active non-marriage" — the
        // pre-existing rose-heart treatment.
        //
        // `directlyAdjacent` is computed here (not in the renderer) because
        // it needs the full positioned-nodes set. A pair is "adjacent"
        // when no other positioned node sits on the same y row strictly
        // between `left.x` and `right.x` — i.e. the glyph at the midpoint
        // is visible to the user and the dashed line behind it would be
        // redundant. "Long" partnerships routed past an intermediate
        // same-row member (Klaus↔Karin past Brigitte; Klaus↔Yuki past
        // Anna) need the line because the midpoint glyph hides behind
        // the intermediate node — only the line conveys the relationship.
        const directlyAdjacent = !positionedList.some(
            (n) => n.id !== left.id && n.id !== right.id && n.y === left.y && n.x > left.x && n.x < right.x,
        )
        return [
            {
                aId: leftIsA ? e.a : e.b,
                bId: leftIsA ? e.b : e.a,
                ax: left.x + NODE_W,
                ay: left.y + NODE_H / 2,
                bx: right.x,
                by: right.y + NODE_H / 2,
                kind: e.kind ?? null,
                ended: e.ended_on !== null && e.ended_on !== undefined && e.ended_on !== '',
                directlyAdjacent,
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

/**
 * Internal: run a single placement pass against the current root order and
 * each block's current `members` shape. Returns the freshly placed map so
 * the outer function can compare passes / iterate.
 */
function runPlacement(
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
 */
function reorderRootsByBarycenter(
    rootBlocks: Block[],
    placed: Map<string, PositionedBlock>,
    childrenOfBlock: Map<string, Block[]>,
): boolean {
    if (rootBlocks.length < 2) return false
    const before = rootBlocks.map((b) => b.id).join('|')
    const barycenters = new Map<string, number>()
    for (const root of rootBlocks) {
        const descendants = collectDescendantBlocks(root, childrenOfBlock)
        let sum = 0
        let count = 0
        for (const d of descendants) {
            const p = placed.get(d.id)
            if (p === undefined) continue
            sum += p.x + p.pixelWidth / 2
            count += 1
        }
        // Roots with NO descendants (orphan leaves) keep their existing x as
        // the barycenter so they don't all collapse to 0 and reshuffle the
        // surviving roots.
        if (count === 0) {
            const own = placed.get(root.id)
            sum = own === undefined ? 0 : own.x + own.pixelWidth / 2
            count = 1
        }
        barycenters.set(root.id, sum / count)
    }
    rootBlocks.sort((a, b) => {
        const ba = barycenters.get(a.id) ?? 0
        const bb = barycenters.get(b.id) ?? 0
        return ba - bb
    })
    return rootBlocks.map((b) => b.id).join('|') !== before
}

function collectDescendantBlocks(root: Block, childrenOfBlock: Map<string, Block[]>): Block[] {
    const out: Block[] = []
    const stack = [...(childrenOfBlock.get(root.id) ?? [])]
    while (stack.length > 0) {
        const next = stack.pop()
        if (next === undefined) continue
        out.push(next)
        for (const c of childrenOfBlock.get(next.id) ?? []) stack.push(c)
    }
    return out
}

/**
 * For each 2-person couple block whose members come from DIFFERENT parent
 * blocks placed on opposite sides of the row above, reverse the member
 * order so each spouse sits closer to their own parents. Mutates the
 * blocks in place. Returns `true` if any block was reversed so the caller
 * can re-run placement.
 */
function swapTwoPersonCouplesByParentX(
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
