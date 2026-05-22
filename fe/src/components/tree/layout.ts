// SVG layout for the family tree. We build a generational tree via d3-hierarchy
// (one canonical parent per child so we have a proper tree) for **x** positions
// only — sibling order + partner adjacency benefit from d3-hierarchy's well
// tuned spacing. The **y** rank is computed independently from generation
// (descendant-depth + an eldest-orphan promotion pass), so that a parentless
// 1910 ancestor lands ABOVE a 1935 row even though it has no canonical parent
// in the tree. Extra (non-canonical) parent edges are still rendered as
// straight lines from the child up to each parent.

import { hierarchy, tree as d3tree, type HierarchyPointNode } from 'd3-hierarchy'

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
// Years per "generation" when promoting an eldest orphan. A 1910 person seen
// against a 1935 row sits one row up (25y gap); a 1885 person two rows up.
// Round half-up so a 12-year gap still nudges, but a 5-year gap does not.
const YEARS_PER_GENERATION = 25

// LayoutData carries the original person id (or `null` for the virtual root)
// through the d3-hierarchy pipeline. The shape must be a recursive tree the
// `hierarchy()` constructor can walk via the default `(d) => d.children`
// accessor, so `children` lives on the data node itself, not a wrapping object.
interface LayoutData {
    id: string | null
    children?: LayoutData[]
}

function buildLayoutNode(id: string | null, childrenOf: Map<string | null, string[]>): LayoutData {
    const childIds = childrenOf.get(id) ?? []
    const children = childIds.map((c) => buildLayoutNode(c, childrenOf))
    if (children.length === 0) {
        return { id }
    }
    return { id, children }
}

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

/**
 * Compute SVG-ready positions and edge coordinates for the family tree.
 *
 * Strategy:
 *   1. Pick one canonical parent per child (smallest parent_id, so the choice
 *      is stable across reloads) — d3-hierarchy needs a true tree. Used for
 *      sibling-ordering x positions only.
 *   2. Insert a virtual root above every parentless person; tree() needs a
 *      single root.
 *   3. Run d3-hierarchy's `tree()` to assign x by sibling order. Discard the
 *      depth-derived y — we replace it with the generation index below.
 *   4. Compute generation per person bottom-up from the FULL parent_edges
 *      adjacency; promote eldest orphans by birth-year gap.
 *   5. Override y with `(topGen - generation) * (NODE_H + ROW_GAP)` so the
 *      highest generation sits at y=0 (top of screen).
 *   6. Partner pass: average each in-row partner pair's x to sit them
 *      side-by-side. Cross-row partners stay where layout put them; the edge
 *      drawer handles them as long lines.
 *   7. Shift all x by -minX so the leftmost node lands at x=0.
 */
export function layoutTree(input: TreeInput): LayoutResult {
    const byId = new Map(input.nodes.map((n) => [n.id, n]))

    // Step 1: canonical parent per child (sorted parent_ids => smallest wins).
    const canonicalParent = new Map<string, string | null>()
    for (const n of input.nodes) {
        const sorted = [...n.parent_ids].sort()
        canonicalParent.set(n.id, sorted[0] ?? null)
    }

    // Step 2: child lists per canonical parent (including the virtual `null`
    // root). This map drives d3-hierarchy's x layout only.
    const canonicalChildrenOf = new Map<string | null, string[]>()
    for (const n of input.nodes) {
        const p = canonicalParent.get(n.id) ?? null
        const list = canonicalChildrenOf.get(p) ?? []
        list.push(n.id)
        canonicalChildrenOf.set(p, list)
    }

    const root = buildLayoutNode(null, canonicalChildrenOf)
    const h = hierarchy<LayoutData>(root)
    const layoutFn = d3tree<LayoutData>().nodeSize([NODE_W + COL_GAP, NODE_H + ROW_GAP])
    const laidOut = layoutFn(h)

    // Step 4: full parent-edge adjacency, so step / poly / multi-parent
    // relationships all count toward generation depth. `a` = child, `b` =
    // parent in EdgePair. Filter to known nodes so a stale edge can't
    // poison the recursion.
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

    // Step 3 + 5: materialize real-node positions; skip the virtual root.
    // x comes from d3-hierarchy; y is overridden by generation rank.
    const positioned = new Map<string, Positioned>()
    laidOut.each((pn: HierarchyPointNode<LayoutData>) => {
        const id = pn.data.id
        if (id === null) return
        const node = byId.get(id)
        if (node === undefined) return
        const gen = generation.get(id) ?? 0
        positioned.set(id, {
            id,
            given_name: node.given_name,
            family_name: node.family_name,
            birth_date: node.birth_date ?? null,
            death_date: node.death_date ?? null,
            linked_user_id: node.linked_user_id ?? null,
            x: pn.x,
            // Highest generation at the top of the canvas (y=0); each step
            // down moves one row of node-height + row-gap.
            y: (topGen - gen) * (NODE_H + ROW_GAP),
        })
    })

    // Step 6: partner pass — pull open in-row pairs adjacent.
    for (const e of input.partner_edges) {
        const a = positioned.get(e.a)
        const b = positioned.get(e.b)
        if (a === undefined || b === undefined) continue
        if (a.y !== b.y) continue
        const mid = (a.x + b.x) / 2
        const half = (NODE_W + COL_GAP) / 2
        if (a.x < b.x) {
            a.x = mid - half
            b.x = mid + half
        } else {
            b.x = mid - half
            a.x = mid + half
        }
    }

    // Step 7: shift so minX = 0; collect bounds.
    let minX = Infinity
    let maxX = -Infinity
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
