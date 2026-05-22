// Generation (vertical row) computation. v3: top-down longest-path depth
// from parentless anchors, with partner equalization and upward
// re-propagation iterated to a fixed point. Compared to the v2 bottom-up
// `max(child.gen) + 1` pass, this lines up co-parents (and therefore
// siblings) on a single row regardless of how deep each parent's branch
// happens to reach — which is the whole "Felix sits with his half-siblings"
// invariant the user reported.

import { type BackendNode, YEARS_PER_GENERATION } from './types'

/**
 * Parse the year out of a (possibly partial) ISO date string. Accepts the
 * full `YYYY-MM-DD` shape SQLx emits as well as the date-only `YYYY` shape
 * the seeded fixtures sometimes carry. Returns `null` for anything we can't
 * confidently read as a 4-digit year — callers treat `null` as "no birth
 * date" and skip those people during the eldest-orphan pass.
 */
export function parseBirthYear(date: string | null | undefined): number | null {
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
export function birthSortKey(date: string | null | undefined): [number, string] {
    const yr = parseBirthYear(date)
    if (yr === null) return [Number.POSITIVE_INFINITY, '']
    return [yr, date ?? '']
}

/**
 * Compute a depth-from-anchors map for every person. Anchors (parentless
 * persons) sit at depth 0; a child sits at `max(parent.depth) + 1` over
 * all `parent_links`. Partner edges then equalize their two endpoints to
 * the max of the pair, and any partner that gets bumped propagates the
 * constraint up through its own parents (a parent must stay strictly
 * above its child, so `parent.depth ≥ child.depth - 1`). The whole thing
 * iterates to a fixed point. Cycle protection: visiting an in-progress
 * id collapses to 0, so a stale payload with a cycle never stack-overflows.
 *
 * Returns the **inverted** generation map: `gen = maxDepth - depth`. That
 * matches the layout convention where the top row is `gen == maxGen` and
 * the canvas y is `(maxGen - gen) * ROW_GAP`, which collapses to the
 * familiar `depth * ROW_GAP`. The inversion happens here (not in the
 * caller) so all of `index.ts` keeps reading `generation` as "row index".
 */
export function computeGenerations(
    nodeIds: string[],
    parentsOfPerson: Map<string, string[]>,
    partnerEdges: ReadonlyArray<{ a: string; b: string }>,
): Map<string, number> {
    const depth = new Map<string, number>()
    const inProgress = new Set<string>()

    // Recursive top-down: depth(id) = max(depth(parent)) + 1 for any parent
    // in `parentsOfPerson`, else 0. Memoized; cycle protection collapses any
    // back-edge to 0 so we never recurse forever.
    function topDown(id: string): number {
        const memo = depth.get(id)
        if (memo !== undefined) return memo
        if (inProgress.has(id)) return 0
        inProgress.add(id)
        const parents = parentsOfPerson.get(id) ?? []
        let best = 0
        for (const p of parents) {
            const pd = topDown(p) + 1
            if (pd > best) best = pd
        }
        inProgress.delete(id)
        depth.set(id, best)
        return best
    }

    for (const id of nodeIds) topDown(id)

    // Skip partner equalization between two persons that already have a
    // direct ancestor relationship. Equalizing a parent-child pair would
    // race the parent constraint forever (each round bumps the child up to
    // match the parent, then the parent up to stay above the child, then
    // the child to match…). A partner who is also their partner's parent
    // is biologically wrong but the algorithm shouldn't loop. We compute
    // ancestor sets ONCE from the parent_edges before iterating.
    const ancestorsOf = new Map<string, Set<string>>()
    function ancestorsFor(id: string, seen: Set<string>): Set<string> {
        const memo = ancestorsOf.get(id)
        if (memo !== undefined) return memo
        if (seen.has(id)) return new Set()
        seen.add(id)
        const out = new Set<string>()
        for (const p of parentsOfPerson.get(id) ?? []) {
            out.add(p)
            for (const a of ancestorsFor(p, seen)) out.add(a)
        }
        seen.delete(id)
        ancestorsOf.set(id, out)
        return out
    }
    for (const id of nodeIds) ancestorsFor(id, new Set())
    const isAncestorRelated = (a: string, b: string): boolean =>
        (ancestorsOf.get(a)?.has(b) ?? false) || (ancestorsOf.get(b)?.has(a) ?? false)
    const eligiblePartnerEdges = partnerEdges.filter((e) => !isAncestorRelated(e.a, e.b))

    // Partner equalize + upward propagate. Bounded by O(V) per pass; we cap
    // the outer loop at `nodeIds.length` iterations as a runaway-safety
    // guard — the graph is a DAG so the algorithm provably terminates in at
    // most `maxDepth` rounds, far below the cap.
    const maxRounds = Math.max(nodeIds.length, 1)
    for (let round = 0; round < maxRounds; round += 1) {
        let changed = false

        // Re-derive each person's depth from its parents in case a partner
        // bump on the previous round moved a parent up. We never DECREASE a
        // depth here — a partner-eq bump from the previous round must
        // survive even if the parent chain alone would suggest a lower row.
        for (const id of nodeIds) {
            const parents = parentsOfPerson.get(id) ?? []
            let want = 0
            for (const p of parents) {
                const pd = (depth.get(p) ?? 0) + 1
                if (pd > want) want = pd
            }
            const cur = depth.get(id) ?? 0
            if (want > cur) {
                depth.set(id, want)
                changed = true
            }
        }

        // Partner equalize: pull each pair up to the larger of the two.
        for (const e of eligiblePartnerEdges) {
            const da = depth.get(e.a)
            const db = depth.get(e.b)
            if (da === undefined || db === undefined) continue
            const m = da > db ? da : db
            if (da < m) {
                depth.set(e.a, m)
                changed = true
            }
            if (db < m) {
                depth.set(e.b, m)
                changed = true
            }
        }

        // Upward parent constraint: every parent must sit at least one row
        // above its child. After a partner-eq bump the child may now be
        // higher than (parent + 1); fix the parent up (never down) and let
        // the next round re-derive the rest of the subtree.
        for (const [child, parents] of parentsOfPerson.entries()) {
            const cd = depth.get(child)
            if (cd === undefined) continue
            for (const p of parents) {
                const pd = depth.get(p)
                if (pd === undefined) continue
                if (pd < cd - 1) {
                    depth.set(p, cd - 1)
                    changed = true
                }
            }
        }

        if (!changed) break
    }

    // Invert: `gen = maxDepth - depth` so layout consumers can keep using
    // the "higher gen == higher on canvas" convention they had under v2.
    let maxDepth = 0
    for (const d of depth.values()) if (d > maxDepth) maxDepth = d
    const generation = new Map<string, number>()
    for (const id of nodeIds) {
        const d = depth.get(id) ?? 0
        generation.set(id, maxDepth - d)
    }
    return generation
}

/**
 * Eldest-orphan promotion. A parentless leaf (no children, no parent_links)
 * with a birth_date older than the median birth_year of the current top
 * generation row gets bumped up by one row per ~25-year gap. This is what
 * lets a 1910 ancestor sit ABOVE a 1935 cohort even though there is no
 * parent edge to hang them from.
 *
 * Returns a new map — does not mutate the input.
 */
export function promoteEldestOrphans(
    nodes: BackendNode[],
    generation: Map<string, number>,
    childrenOfPerson: Map<string, string[]>,
): Map<string, number> {
    const promoted = new Map(generation)
    let topGen = 0
    for (const g of promoted.values()) if (g > topGen) topGen = g

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
        topYears.length % 2 === 0 ? ((topYears[mid - 1] ?? 0) + (topYears[mid] ?? 0)) / 2 : (topYears[mid] ?? 0)

    for (const n of nodes) {
        if (promoted.get(n.id) !== topGen) continue
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
