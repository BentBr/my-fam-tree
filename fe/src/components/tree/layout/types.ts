// Shared types + spacing constants for the SVG family-tree layout. Imported
// by both the layout pipeline and the Vue components so the math and the
// rendered SVG agree on a single source of node-size + gap values.

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
export const ROW_GAP = 100
export const COL_GAP = 24
// Wider gap between sibling-clusters of *different* parent blocks. Inside one
// parent's children we use the standard COL_GAP; between two adjacent parent
// groups we want the visual grouping to be unambiguous so the user can tell
// "these three siblings belong to that couple" at a glance.
export const CLUSTER_GAP = COL_GAP * 2
// Years per "generation" when promoting an eldest orphan. A 1910 person seen
// against a 1935 row sits one row up (25y gap); a 1885 person two rows up.
// Round half-up so a 12-year gap still nudges, but a 5-year gap does not.
export const YEARS_PER_GENERATION = 25

// A "block" is the placement unit on each row. Either a single person
// (`members.length === 1`) or a couple — two same-row partners drawn
// side-by-side. The block.id is a stable string derived from the member
// ids so we can key parent-of relations on it.
export interface Block {
    id: string
    members: string[] // 1 or 2 person ids, left-to-right
    /** Y row (already in pixel space; same value for all members of the block). */
    y: number
    /** Number of person columns this block occupies (1 or 2). */
    width: number
}

export interface PositionedBlock extends Block {
    /** x of the LEFT edge of the leftmost member. */
    x: number
}
