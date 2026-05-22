// Generation (vertical row) computation. The y-rank of each person comes
// from the *full* parent-edge adjacency (bottom-up max-of-children), not
// from the canonical-parent subset, so step / poly / multi-parent
// relationships all push ancestors up to where the user expects them.

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
 * Compute a generation index for every person, bottom-up over the full
 * parent-edge adjacency (not the canonical-parent subset). Leaves get `0`;
 * a person sits at `1 + max(gen(child))`. Cycle protection: any node that
 * recurses into itself collapses to `0` — the backend should reject cycles
 * via its DB trigger, but we never want a runtime stack overflow if a stale
 * payload sneaks one through.
 */
export function computeGenerations(nodeIds: string[], childrenOfPerson: Map<string, string[]>): Map<string, number> {
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
        if (promoted.get(n.id) !== 0) continue
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
