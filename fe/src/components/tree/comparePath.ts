// Path highlighting for compare mode.

/** What the canvas lights for the active kinship relation. */
export interface PathHighlight {
    /** ORDERED person ids, A … B. */
    path: readonly string[]
    /** The one partnership the path crosses, or null for a blood relation. */
    partnershipId: string | null
}

/** Orientation-independent key for a pair of persons. */
export function edgeKey(a: string, b: string): string {
    return a < b ? `${a}|${b}` : `${b}|${a}`
}

/**
 * Keys of the pairs of ADJACENT path members. A plain id set would also
 * match a "chord" between two non-adjacent members.
 */
export function pathEdgeKeys(path: readonly string[]): Set<string> {
    const keys = new Set<string>()
    path.forEach((id, i) => {
        const next = path[i + 1]
        if (next !== undefined) keys.add(edgeKey(id, next))
    })
    return keys
}
