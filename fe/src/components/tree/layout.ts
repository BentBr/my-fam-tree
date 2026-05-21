// SVG layout for the family tree. We build a generational tree via d3-hierarchy
// (one canonical parent per child so we have a proper tree), then run a partner
// pass that nudges open partner pairs onto adjacent x coordinates on the same
// row. Extra (non-canonical) parent edges are still rendered as straight lines
// from the child up to each parent.

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
 * Compute SVG-ready positions and edge coordinates for the family tree.
 *
 * Strategy:
 *   1. Pick one canonical parent per child (smallest parent_id, so the choice
 *      is stable across reloads) — d3-hierarchy needs a true tree.
 *   2. Insert a virtual root above every parentless person; tree() needs a
 *      single root.
 *   3. Run d3-hierarchy's `tree()` to assign x by sibling order, y by depth.
 *   4. Partner pass: average each in-row partner pair's x to sit them
 *      side-by-side. Cross-row partners stay where layout put them; the edge
 *      drawer handles them as long lines.
 *   5. Shift all x by -minX so the leftmost node lands at x=0.
 */
export function layoutTree(input: TreeInput): LayoutResult {
    const byId = new Map(input.nodes.map((n) => [n.id, n]))

    // Step 1: canonical parent per child (sorted parent_ids => smallest wins).
    const canonicalParent = new Map<string, string | null>()
    for (const n of input.nodes) {
        const sorted = [...n.parent_ids].sort()
        canonicalParent.set(n.id, sorted[0] ?? null)
    }

    // Step 2: child lists per parent (including the virtual `null` root).
    const childrenOf = new Map<string | null, string[]>()
    for (const n of input.nodes) {
        const p = canonicalParent.get(n.id) ?? null
        const list = childrenOf.get(p) ?? []
        list.push(n.id)
        childrenOf.set(p, list)
    }

    const root = buildLayoutNode(null, childrenOf)
    const h = hierarchy<LayoutData>(root)
    const layoutFn = d3tree<LayoutData>().nodeSize([NODE_W + COL_GAP, NODE_H + ROW_GAP])
    const laidOut = layoutFn(h)

    // Step 3: materialize real-node positions; skip the virtual root.
    const positioned = new Map<string, Positioned>()
    laidOut.each((pn: HierarchyPointNode<LayoutData>) => {
        const id = pn.data.id
        if (id === null) return
        const node = byId.get(id)
        if (node === undefined) return
        positioned.set(id, {
            id,
            given_name: node.given_name,
            family_name: node.family_name,
            birth_date: node.birth_date ?? null,
            death_date: node.death_date ?? null,
            linked_user_id: node.linked_user_id ?? null,
            x: pn.x,
            // Hide the virtual root row by subtracting 1.
            y: (pn.depth - 1) * (NODE_H + ROW_GAP),
        })
    })

    // Step 4: partner pass — pull open in-row pairs adjacent.
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

    // Step 5: shift so minX = 0; collect bounds.
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
