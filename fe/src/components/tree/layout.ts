// SVG layout for the family tree. v2: block-based layout where each
// "couple" (two same-row partners) is the placement unit. Children sit
// centered under their parent block; sibling groups of different parent
// blocks are separated by a wider CLUSTER_GAP so the visual grouping is
// unambiguous. Top-row blocks are laid out left-to-right in stable id
// order. We compute the generation rank independently from the canonical
// parent edge so that parentless older ancestors still land above younger
// rows.

// Field shapes mirror the wire format from `/api/v1/relationships`. `birth_date`
// and `death_date` are optional **and** nullable on the wire (utoipa emits
// `Option<NaiveDate>` as `string | null | undefined`), so the FE layout sees
// `undefined` when the column was never set and `null` when it was explicitly
// cleared. Both collapse to "unknown" in the rendered date label.
export interface BackendNode {
    id: string
    given_name: string
    family_name: string
    birth_date?: string | null
    death_date?: string | null
    linked_user_id?: string | null
    parent_ids: string[]
    partner_ids: string[]
}

export interface BackendEdge {
    a: string
    b: string
}

export interface TreeInput {
    nodes: BackendNode[]
    parent_edges: BackendEdge[]
    partner_edges: BackendEdge[]
}

export interface Positioned {
    id: string
    given_name: string
    family_name: string
    birth_date: string | null
    death_date: string | null
    linked_user_id: string | null
    x: number
    y: number
}

export interface ParentEdge {
    childId: string
    parentId: string
    childX: number
    childY: number
    parentX: number
    parentY: number
}

export interface PartnerEdge {
    aId: string
    bId: string
    ax: number
    ay: number
    bx: number
    by: number
}

export interface LayoutResult {
    nodes: Positioned[]
    parentEdges: ParentEdge[]
    partnerEdges: PartnerEdge[]
    width: number
    height: number
}

// These constants mirror the SVG rendering in TreeNode.vue. Keeping a single
// source for spacing avoids drift between layout math and visuals.
export const NODE_W = 200
export const NODE_H = 72
const ROW_GAP = 100
const COL_GAP = 24
// Wider gap between sibling-clusters of *different* parent blocks. Inside one
// parent's children we use the standard COL_GAP; between two adjacent parent
// groups we want the visual grouping to be unambiguous so the user can tell
// "these three siblings belong to that couple" at a glance.
const CLUSTER_GAP = COL_GAP * 2
// Years per "generation" when promoting an eldest orphan. A 1910 person seen
// against a 1935 row sits one row up (25y gap); a 1885 person two rows up.
// Round half-up so a 12-year gap still nudges, but a 5-year gap does not.
const YEARS_PER_GENERATION = 25

/**
 * Parse the year out of a (possibly partial) ISO date string. Accepts the
 * full `YYYY-MM-DD` shape SQLx emits as well as the date-only `YYYY` shape
 * the seeded fixtures sometimes carry. Returns `null` for anything we can't
 * confidently read as a 4-digit year — callers treat `null` as "no birth
 * date" and skip those people during the eldest-orphan pass.
 */
function parseBirthYear(date: string | null | undefined): number | null {
    if (date === null || date === undefined || date === '') return null
    const m = /^(\d{4})/.exec(date)
    if (m === null) return null
    const head = m[1]
    if (head === undefined) return null
    const yr = Number.parseInt(head, 10)
    return Number.isFinite(yr) ? yr : null
}

/**
 * Numeric sort key for birth date strings. Missing/invalid dates sort to
 * the end (Infinity) so children with known birth dates always come first
 * within a sibling group. Falls back to the full ISO string for tie-breaks
 * inside the same year — `1990-01-12` < `1990-02-01` and so on.
 */
function birthSortKey(date: string | null | undefined): [number, string] {
    const yr = parseBirthYear(date)
    if (yr === null) return [Number.POSITIVE_INFINITY, '']
    return [yr, date ?? '']
}

/**
 * Compute a generation index for every person, bottom-up over the full
 * parent-edge adjacency (not the canonical-parent subset). Leaves get `0`;
 * a person sits at `1 + max(gen(child))`. Cycle protection: any node that
 * recurses into itself collapses to `0` — the backend should reject cycles
 * via its DB trigger, but we never want a runtime stack overflow if a stale
 * payload sneaks one through.
 */
function computeGenerations(nodeIds: string[], childrenOfPerson: Map<string, string[]>): Map<string, number> {
    const generation = new Map<string, number>()
    const inProgress = new Set<string>()

    function visit(id: string): number {
        const memo = generation.get(id)
        if (memo !== undefined) return memo
        if (inProgress.has(id)) return 0
        inProgress.add(id)
        const kids = childrenOfPerson.get(id) ?? []
        let best = 0
        for (const c of kids) {
            const cg = visit(c) + 1
            if (cg > best) best = cg
        }
        inProgress.delete(id)
        generation.set(id, best)
        return best
    }

    for (const id of nodeIds) visit(id)
    return generation
}

/**
 * Eldest-orphan promotion. A parentless leaf (no children, no parent_links)
 * with a birth_date older than the median birth_year of the current top
 * generation row gets bumped up by one row per ~25-year gap. This is what
 * lets a 1910 ancestor sit ABOVE a 1935 cohort even though d3-hierarchy
 * has no parent edge to hang them from.
 *
 * Returns a new map — does not mutate the input.
 */
function promoteEldestOrphans(
    nodes: BackendNode[],
    generation: Map<string, number>,
    childrenOfPerson: Map<string, string[]>,
): Map<string, number> {
    const promoted = new Map(generation)
    let topGen = 0
    for (const g of promoted.values()) if (g > topGen) topGen = g

    // Median birth_year of the current top row. If nobody on the top row
    // has a birth date we have no signal to anchor against — skip the pass.
    const topYears: number[] = []
    for (const n of nodes) {
        if (promoted.get(n.id) !== topGen) continue
        const y = parseBirthYear(n.birth_date)
        if (y !== null) topYears.push(y)
    }
    if (topYears.length === 0) return promoted
    topYears.sort((a, b) => a - b)
    const mid = Math.floor(topYears.length / 2)
    const medianYear =
        topYears.length % 2 === 0
            ? // Average for an even count. Both indices are in range because
              // length > 0 and mid > 0 in the even branch.
              ((topYears[mid - 1] ?? 0) + (topYears[mid] ?? 0)) / 2
            : (topYears[mid] ?? 0)

    for (const n of nodes) {
        if (promoted.get(n.id) !== 0) continue
        // Only "orphan leaves" — no children AND no parents — are candidates.
        // Someone with a parent already has a structural row; bumping them
        // would crash through the parent edge.
        const kids = childrenOfPerson.get(n.id) ?? []
        if (kids.length > 0) continue
        if (n.parent_ids.length > 0) continue
        const year = parseBirthYear(n.birth_date)
        if (year === null) continue
        if (year >= medianYear) continue
        const gap = medianYear - year
        const extra = Math.max(1, Math.round(gap / YEARS_PER_GENERATION))
        promoted.set(n.id, topGen + extra)
    }

    return promoted
}

// A "block" is the placement unit on each row. Either a single person
// (`members.length === 1`) or a couple — two same-row partners drawn
// side-by-side. The block.id is a stable string derived from the member
// ids so we can key parent-of relations on it.
interface Block {
    id: string
    members: string[] // 1 or 2 person ids, left-to-right
    /** Y row (already in pixel space; same value for all members of the block). */
    y: number
    /** Number of person columns this block occupies (1 or 2). */
    width: number
}

interface PositionedBlock extends Block {
    /** x of the LEFT edge of the leftmost member. */
    x: number
}

/**
 * Build the per-generation block list. For each row we walk the row members
 * in stable id order and pair anyone partnered to a same-row peer that hasn't
 * already been paired. Everyone else becomes a singleton block.
 */
function buildBlocks(
    nodeIds: string[],
    generation: Map<string, number>,
    partnerOf: Map<string, Set<string>>,
): Map<number, Block[]> {
    // Group ids by row, sorted stably for determinism. Stable id order lets
    // the top-row layout (which has no canonical-parent anchor) repaint
    // identically across reloads.
    const byRow = new Map<number, string[]>()
    for (const id of nodeIds) {
        const g = generation.get(id) ?? 0
        const row = byRow.get(g) ?? []
        row.push(id)
        byRow.set(g, row)
    }
    for (const row of byRow.values()) row.sort()

    const blocks = new Map<number, Block[]>()
    for (const [g, ids] of byRow.entries()) {
        const consumed = new Set<string>()
        const list: Block[] = []
        for (const id of ids) {
            if (consumed.has(id)) continue
            const partners = partnerOf.get(id) ?? new Set<string>()
            // Pick the smallest-id same-row partner that hasn't been paired yet.
            let mate: string | null = null
            for (const p of partners) {
                if (consumed.has(p)) continue
                if ((generation.get(p) ?? -1) !== g) continue
                if (mate === null || p < mate) mate = p
            }
            if (mate !== null) {
                consumed.add(id)
                consumed.add(mate)
                // Left member is the smaller id for stable visuals.
                const left = id < mate ? id : mate
                const right = id < mate ? mate : id
                list.push({
                    id: `couple:${left}|${right}`,
                    members: [left, right],
                    y: 0, // filled in later
                    width: 2,
                })
            } else {
                consumed.add(id)
                list.push({
                    id: `single:${id}`,
                    members: [id],
                    y: 0,
                    width: 1,
                })
            }
        }
        blocks.set(g, list)
    }
    return blocks
}

/**
 * Choose a canonical parent block for each non-top block. A block hangs from
 * one parent block (the block that contains its canonical parent person);
 * extra parent edges still render as straight lines but don't influence
 * placement. Couples inherit the canonical parent of their LEFT member,
 * which keeps the tree shape predictable when both partners have known
 * ancestors.
 */
function chooseParentBlock(
    block: Block,
    blockOfPerson: Map<string, Block>,
    nodeById: Map<string, BackendNode>,
): Block | null {
    const anchorId = block.members[0]
    if (anchorId === undefined) return null
    const anchor = nodeById.get(anchorId)
    if (anchor === undefined) return null
    const sortedParents = [...anchor.parent_ids].sort()
    for (const pid of sortedParents) {
        const pb = blockOfPerson.get(pid)
        if (pb !== undefined) return pb
    }
    return null
}

/**
 * Compute the natural sort key for a block — used to order both root blocks
 * and sibling blocks (children of the same parent). Couples sort by their
 * left member's birth_date so the *oldest* of the pair anchors the order;
 * within ties we fall back to the left member's id for stability.
 */
function blockSortKey(block: Block, nodeById: Map<string, BackendNode>): [number, string, string] {
    const leftId = block.members[0]
    if (leftId === undefined) return [Number.POSITIVE_INFINITY, '', block.id]
    const n = nodeById.get(leftId)
    const [yr, iso] = birthSortKey(n?.birth_date)
    return [yr, iso, block.id]
}

function compareBlockKeys(a: [number, string, string], b: [number, string, string]): number {
    if (a[0] !== b[0]) return a[0] - b[0]
    if (a[1] !== b[1]) return a[1] < b[1] ? -1 : 1
    return a[2] < b[2] ? -1 : a[2] > b[2] ? 1 : 0
}

/**
 * Recursive subtree placement. Returns the placed x-extent (`[xL, xR]`) of
 * the subtree rooted at `block`. The recursion:
 *   1. Lays out all of `block`'s children first, left-to-right with COL_GAP
 *      inside the cluster.
 *   2. Centers `block` on the children's mid-point. If the children cluster
 *      is narrower than the block itself, the block extends past the cluster
 *      on either side — the caller compensates with CLUSTER_GAP between
 *      sibling clusters.
 *   3. If the block has no children, it gets placed at the leftmost free x
 *      passed in by the caller via the `cursor` ref.
 *
 * Each block contributes `width * NODE_W + (width - 1) * COL_GAP` columns.
 */
function layoutSubtree(
    block: Block,
    childrenOfBlock: Map<string, Block[]>,
    nodeById: Map<string, BackendNode>,
    placed: Map<string, PositionedBlock>,
    cursor: { x: number },
    rowStep: number,
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

    // Place each child subtree in turn, using COL_GAP between adjacent
    // children. The cursor advances naturally as each subtree consumes
    // its width.
    let firstL = Number.POSITIVE_INFINITY
    let lastR = Number.NEGATIVE_INFINITY
    for (let i = 0; i < sortedChildren.length; i += 1) {
        if (i > 0) cursor.x += COL_GAP
        const child = sortedChildren[i]
        if (child === undefined) continue
        const { xL, xR } = layoutSubtree(child, childrenOfBlock, nodeById, placed, cursor, rowStep)
        if (xL < firstL) firstL = xL
        if (xR > lastR) lastR = xR
    }
    if (!Number.isFinite(firstL)) {
        // Defensive — shouldn't happen because we checked `children.length`.
        const xL = cursor.x
        placed.set(block.id, { ...block, x: xL })
        cursor.x = xL + blockWidth
        return { xL, xR: cursor.x }
    }

    // Center the block over its children. If the resulting block extends
    // left past the cluster's leftmost child, we shift the entire subtree
    // right so the block's left edge sits at the original `firstL`. This
    // preserves the per-row "no overlap" invariant by guaranteeing the
    // block never reaches into the previous sibling's space.
    const childrenMid = (firstL + lastR) / 2
    let blockL = childrenMid - blockWidth / 2
    if (blockL < firstL) {
        const delta = firstL - blockL
        // Shift every child placement inside this subtree right by `delta`.
        for (const child of sortedChildren) {
            shiftSubtree(child, childrenOfBlock, placed, delta)
        }
        blockL = firstL
        // cursor advanced based on the shifted children's new right edge.
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
function shiftSubtree(
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
 *   4. Place top-row blocks left-to-right in stable id order with
 *      CLUSTER_GAP between them. Recursively lay out each top block's
 *      subtree: children are placed left-to-right by birth date, then the
 *      parent block is centered over the children cluster.
 *   5. Materialize positioned persons by reading each block's x and
 *      distributing it over the 1 or 2 members.
 *   6. Build parent + partner edges from the placed coordinates. Partner
 *      edges between same-row members of one block are short adjacent
 *      lines by construction.
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

    // Build child-of-block adjacency: for each non-top block, find its
    // canonical parent block. A block with no canonical parent block is a
    // "root" of its own subtree (placed at the top level in id order).
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
                // No parent on the canvas — treat as a root anchored at this row.
                rootBlocks.push(b)
                continue
            }
            const kids = childrenOfBlock.get(pb.id) ?? []
            kids.push(b)
            childrenOfBlock.set(pb.id, kids)
        }
    }

    // Sort root blocks: top-row by birth date (oldest first) then id;
    // mid-row roots (orphans of a non-top block) follow the same key. The
    // ordering anchors visual stability across reloads.
    rootBlocks.sort((a, b) => compareBlockKeys(blockSortKey(a, byId), blockSortKey(b, byId)))

    // Place every root subtree left-to-right with CLUSTER_GAP between them.
    const placed = new Map<string, PositionedBlock>()
    const cursor = { x: 0 }
    for (let i = 0; i < rootBlocks.length; i += 1) {
        if (i > 0) cursor.x += CLUSTER_GAP
        const root = rootBlocks[i]
        if (root === undefined) continue
        layoutSubtree(root, childrenOfBlock, byId, placed, cursor, rowStep)
    }

    // Per-row separation pass. The recursive layout guarantees no overlap
    // *within* a subtree, but two subtrees on the same row may collide if a
    // descendant cluster is wider than the available top-row slot. Walk
    // each row left-to-right and enforce a hard floor of CLUSTER_GAP
    // between blocks that share no parent block, COL_GAP between siblings.
    // Sibling vs cousin is distinguished by parent-block id.
    const parentOfBlock = new Map<string, string | null>()
    for (const [pid, kids] of childrenOfBlock.entries()) {
        for (const k of kids) parentOfBlock.set(k.id, pid)
    }
    {
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
                    // Shift the curr block and everything in its subtree.
                    const block: Block = curr
                    shiftSubtree(block, childrenOfBlock, placed, delta)
                }
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
