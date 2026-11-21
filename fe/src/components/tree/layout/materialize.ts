// Final stage of `layoutTree`: turn placed blocks into positioned persons,
// normalise to minX == 0 and emit SVG-ready parent + partner edges.

import {
    COL_GAP,
    type LayoutResult,
    NODE_H,
    NODE_W,
    type ParentEdge,
    type PartnerEdge,
    type Positioned,
    type PositionedBlock,
    type TreeInput,
} from './types'

export function materializeLayout(
    input: TreeInput,
    byId: Map<string, TreeInput['nodes'][number]>,
    placed: Map<string, PositionedBlock>,
): LayoutResult {
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
