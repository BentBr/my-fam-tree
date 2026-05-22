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

/**
 * Parent-edge wire shape. `kind` is optional because test fixtures construct
 * bare `{a, b}` pairs; the runtime payload always carries it. The layout
 * uses `kind` to split children of a multi-couple block under the correct
 * bio-couple midpoint — a step-link to a current partner doesn't drag the
 * child under that partnership.
 */
export interface BackendEdge {
    a: string
    b: string
    kind?: string
}

/**
 * Partner-edge wire shape. Carries the lifecycle fields the layout needs to
 * order multi-partner blocks: open partnerships (`ended_on === null`) sit on
 * the right of the shared person, ended ones (`ended_on !== null`) on the
 * left, sorted by `ended_on` ascending so the oldest divorce ends up
 * leftmost. Optional fields keep test fixtures (`{a, b}`) compatible.
 */
export interface BackendPartnerEdge {
    a: string
    b: string
    id?: string
    kind?: string
    started_on?: string | null
    ended_on?: string | null
    end_reason?: string | null
}

export interface TreeInput {
    nodes: BackendNode[]
    parent_edges: BackendEdge[]
    partner_edges: BackendPartnerEdge[]
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

/**
 * A same-row partnership inside a block. `leftIdx` / `rightIdx` are indices
 * into `Block.members` (left < right, adjacent in the threaded order). The
 * `ended` flag drives styling/ordering — open partnerships sit to the right
 * of the shared anchor, ended ones to the left.
 */
export interface BlockCouple {
    leftIdx: number
    rightIdx: number
    ended: boolean
}

// A "block" is the placement unit on each row. Either a single person
// (`members.length === 1`) or N≥2 same-row partners threaded through a shared
// anchor (a 3-member block models "Brigitte (ex), Klaus, Anna (current)"
// where Klaus is the anchor). The block.id is a stable string derived from
// the member ids so we can key parent-of relations on it.
export interface Block {
    id: string
    /** ≥1 person ids, left-to-right. */
    members: string[]
    /**
     * Internal partnerships, one per adjacent member pair that has a partner
     * edge. Empty for singletons. For a plain 2-member couple, exactly one
     * entry. Used by `subtree.ts` to split children under the correct
     * bio-couple midpoint when the block has more than one couple.
     */
    couples: BlockCouple[]
    /** Y row (already in pixel space; same value for all members of the block). */
    y: number
    /** Number of person columns this block occupies (== members.length). */
    width: number
}

export interface PositionedBlock extends Block {
    /** x of the LEFT edge of the leftmost member. */
    x: number
    /**
     * Per-member x offset from the block's left edge. Length === members.length.
     * For a default-spaced block this is `[0, NODE_W+COL_GAP, 2*(NODE_W+COL_GAP), …]`;
     * a multi-couple block whose children sub-clusters would otherwise collide
     * grows its internal gaps so each couple's midpoint can match its
     * sub-cluster midpoint.
     */
    memberOffsets: number[]
    /** Total pixel width including any widened internal gaps. */
    pixelWidth: number
}
