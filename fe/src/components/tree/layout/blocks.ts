// Block construction + sort helpers for layout v2/v3. A block is the
// placement unit on each row: a singleton, a 2-person couple, or — new in
// the ex-spouse-adjacency pass — an N-person chain threaded through a shared
// anchor (e.g. "Brigitte (ex), Klaus, Anna (current)"). The block-building
// algorithm collapses each same-row partner-edge connected component into
// one block so an ex-partner sits adjacent to their shared person instead
// of being banished to a singleton slot at the row's far edge.

import { birthSortKey } from './generations'
import { type BackendNode, type BackendPartnerEdge, type Block, type BlockCouple } from './types'

/** Per-row partner edge with lifecycle metadata used for ordering. */
interface RowEdge {
    a: string
    b: string
    ended_on: string | null
}

function normalizeEnded(e: BackendPartnerEdge): string | null {
    return e.ended_on ?? null
}

/**
 * Sort an anchor's same-row partners ascending by `ended_on`, tiebreaking
 * on partner id for determinism. Callers split open vs ended before
 * calling, so this function only ever sees one cohort at a time.
 */
function sortPartners(
    partners: Array<{ id: string; ended_on: string | null }>,
): Array<{ id: string; ended_on: string | null }> {
    const copy = [...partners]
    copy.sort((a, b) => {
        const aEnd = a.ended_on ?? ''
        const bEnd = b.ended_on ?? ''
        if (aEnd !== bEnd) return aEnd < bEnd ? -1 : 1
        return a.id < b.id ? -1 : a.id > b.id ? 1 : 0
    })
    return copy
}

/**
 * Build the per-generation block list. v3.1: each same-row connected
 * component of the partner graph becomes one block. The component is
 * threaded through the highest-degree anchor person (ties → smallest id) so
 * the anchor sits at the visual centre with their ended partners to the
 * left and open partners to the right.
 */
export function buildBlocks(
    nodeIds: string[],
    generation: Map<string, number>,
    partnerEdges: BackendPartnerEdge[],
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

    // Build a per-row adjacency map from the same-row partner edges. We keep
    // the lifecycle data with each edge so the anchor-ordering pass can
    // partition open vs ended without re-joining against the input.
    const edgesByPerson = new Map<string, RowEdge[]>()
    for (const e of partnerEdges) {
        const ga = generation.get(e.a)
        const gb = generation.get(e.b)
        if (ga === undefined || gb === undefined || ga !== gb) continue
        const rowEdge: RowEdge = { a: e.a, b: e.b, ended_on: normalizeEnded(e) }
        const la = edgesByPerson.get(e.a) ?? []
        la.push(rowEdge)
        edgesByPerson.set(e.a, la)
        const lb = edgesByPerson.get(e.b) ?? []
        lb.push(rowEdge)
        edgesByPerson.set(e.b, lb)
    }

    const blocks = new Map<number, Block[]>()
    for (const [g, ids] of byRow.entries()) {
        const consumed = new Set<string>()
        const list: Block[] = []
        for (const id of ids) {
            if (consumed.has(id)) continue
            const component = collectComponent(id, ids, edgesByPerson, consumed)
            const block = threadComponent(component, id, edgesByPerson)
            for (const m of block.members) consumed.add(m)
            list.push(block)
        }
        // Tag y later; placeholder here.
        for (const b of list) b.y = 0
        blocks.set(g, list)
    }
    return blocks
}

/**
 * BFS the connected component of `seed` inside the row's partner subgraph.
 * Returns the set of person ids reachable through same-row partner edges.
 * `consumed` excludes already-blocked persons except for `seed` itself.
 */
function collectComponent(
    seed: string,
    rowIds: string[],
    edgesByPerson: Map<string, RowEdge[]>,
    consumed: Set<string>,
): Set<string> {
    const rowSet = new Set(rowIds)
    const out = new Set<string>()
    const stack = [seed]
    while (stack.length > 0) {
        const cur = stack.pop()
        if (cur === undefined) continue
        if (out.has(cur)) continue
        if (consumed.has(cur) && cur !== seed) continue
        if (!rowSet.has(cur)) continue
        out.add(cur)
        for (const e of edgesByPerson.get(cur) ?? []) {
            const next = e.a === cur ? e.b : e.a
            if (!out.has(next)) stack.push(next)
        }
    }
    return out
}

/**
 * Turn a same-row partner component into a left-to-right ordered block.
 *
 * Singletons: 1 person, 0 edges → `members: [id]`, `couples: []`.
 *
 * Plain couple: 2 persons, 1 edge → `members: [smallerId, largerId]`, one
 *   `couple` entry covering the pair. Matches the v2 stable visual.
 *
 * Multi-couple chain: pick the anchor (highest-degree person inside the
 *   component, ties → smallest id). Partition the anchor's same-row
 *   partners into open vs ended; ended partners thread to the LEFT (oldest
 *   ended_on furthest from the anchor), open partners thread to the RIGHT.
 *   Any component members not directly adjacent to the anchor land beyond
 *   the open side in stable id order — a safety net for unusual topologies
 *   (multi-hub components); the seed and the common case never hit it.
 */
function threadComponent(component: Set<string>, fallbackSeed: string, edgesByPerson: Map<string, RowEdge[]>): Block {
    const ids = [...component]
    if (ids.length === 1) {
        const only = ids[0] ?? fallbackSeed
        return {
            id: `single:${only}`,
            members: [only],
            couples: [],
            y: 0,
            width: 1,
        }
    }
    if (ids.length === 2) {
        ids.sort()
        const left = ids[0] ?? fallbackSeed
        const right = ids[1] ?? fallbackSeed
        const edge = edgeBetween(left, right, edgesByPerson)
        return {
            id: `couple:${left}|${right}`,
            members: [left, right],
            couples: [{ leftIdx: 0, rightIdx: 1, ended: (edge?.ended_on ?? null) !== null }],
            y: 0,
            width: 2,
        }
    }
    // ≥3 members: pick the anchor (highest degree, lowest id on tie).
    let anchor: string | null = null
    let bestDeg = -1
    for (const id of ids) {
        const deg = (edgesByPerson.get(id) ?? []).length
        if (deg > bestDeg) {
            anchor = id
            bestDeg = deg
        } else if (deg === bestDeg && anchor !== null && id < anchor) {
            anchor = id
        }
    }
    if (anchor === null) anchor = ids[0] ?? fallbackSeed

    const anchorEdges = edgesByPerson.get(anchor) ?? []
    const directPartners = anchorEdges.map((e) => ({
        id: e.a === anchor ? e.b : e.a,
        ended_on: e.ended_on,
    }))
    const open = directPartners.filter((p) => p.ended_on === null)
    const ended = directPartners.filter((p) => p.ended_on !== null)
    const leftOrdered = sortPartners(ended) // oldest ended_on first → leftmost
    const rightOrdered = sortPartners(open)

    const placed = new Set<string>([anchor, ...leftOrdered.map((p) => p.id), ...rightOrdered.map((p) => p.id)])
    const stragglers = ids.filter((id) => !placed.has(id)).sort()

    // `leftOrdered` ascending by ended_on (oldest first). Placed left-to-
    // right that puts the oldest ex at the leftmost slot and the most-recent
    // ex immediately left of the anchor.
    const members = [...leftOrdered.map((p) => p.id), anchor, ...rightOrdered.map((p) => p.id), ...stragglers]

    const couples: BlockCouple[] = []
    for (let i = 0; i < members.length - 1; i += 1) {
        const a = members[i]
        const b = members[i + 1]
        if (a === undefined || b === undefined) continue
        const e = edgeBetween(a, b, edgesByPerson)
        if (e === null) continue
        couples.push({ leftIdx: i, rightIdx: i + 1, ended: e.ended_on !== null })
    }
    return {
        id: `chain:${members.join('|')}`,
        members,
        couples,
        y: 0,
        width: members.length,
    }
}

function edgeBetween(a: string, b: string, edgesByPerson: Map<string, RowEdge[]>): RowEdge | null {
    for (const e of edgesByPerson.get(a) ?? []) {
        if ((e.a === a && e.b === b) || (e.a === b && e.b === a)) return e
    }
    return null
}

/**
 * Choose a canonical parent block for each non-top block. A block hangs from
 * one parent block (the block that contains its canonical parent person);
 * extra parent edges still render as straight lines but don't influence
 * placement. Multi-member blocks inherit the canonical parent of their LEFT
 * member, which keeps the tree shape predictable when both partners have
 * known ancestors.
 */
export function chooseParentBlock(
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
export function blockSortKey(block: Block, nodeById: Map<string, BackendNode>): [number, string, string] {
    const leftId = block.members[0]
    if (leftId === undefined) return [Number.POSITIVE_INFINITY, '', block.id]
    const n = nodeById.get(leftId)
    const [yr, iso] = birthSortKey(n?.birth_date)
    return [yr, iso, block.id]
}

export function compareBlockKeys(a: [number, string, string], b: [number, string, string]): number {
    if (a[0] !== b[0]) return a[0] - b[0]
    if (a[1] !== b[1]) return a[1] < b[1] ? -1 : 1
    return a[2] < b[2] ? -1 : a[2] > b[2] ? 1 : 0
}
