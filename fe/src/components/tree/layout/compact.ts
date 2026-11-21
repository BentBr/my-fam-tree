// Root-cluster compaction for `layoutTree`.

import { type Block, CLUSTER_GAP, type PositionedBlock } from './types'

/**
 * Pull each disjoint root cluster as far LEFT as per-row collisions
 * allow, so a cluster whose only link to the rest of the tree is a
 * married-out child sits close to its connection point instead of being
 * parked at the far-right edge by sequential placement.
 *
 * A "cluster" is a root block plus every block in its block-tree subtree
 * (`childrenOfBlock`). The shift is RIGID — every block in the cluster
 * moves by the same delta — so parent-over-child centering and all
 * within-cluster edges are preserved. Only inter-cluster whitespace
 * shrinks, never below CLUSTER_GAP.
 *
 * Clusters are processed left-to-right (by current min x). The leftmost
 * never moves; each subsequent one packs left against everything already
 * fixed to its left. The bound is computed per-row: for each block in
 * the cluster, the nearest non-cluster block to its left in the same row
 * caps how far the cluster can travel; the min slack across all the
 * cluster's rows is the rigid shift. A child who married out lives in
 * ANOTHER cluster, so it correctly acts as a left-neighbour wall (the
 * cluster packs up to it, not through it) while its cross-cluster edge
 * merely shortens.
 */
export function compactRootClusters(
    rootBlocks: Block[],
    childrenOfBlock: Map<string, Block[]>,
    placed: Map<string, PositionedBlock>,
): void {
    // Build each root's subtree block-id set + whether it owns children.
    const clusters: Array<{ ids: string[]; hasOwnChildren: boolean }> = []
    for (const root of rootBlocks) {
        const ids: string[] = []
        const seen = new Set<string>()
        const stack = [root.id]
        while (stack.length > 0) {
            const id = stack.pop()
            if (id === undefined || seen.has(id)) continue
            seen.add(id)
            ids.push(id)
            for (const c of childrenOfBlock.get(id) ?? []) stack.push(c.id)
        }
        clusters.push({ ids, hasOwnChildren: (childrenOfBlock.get(root.id) ?? []).length > 0 })
    }
    const clusterOf = new Map<string, number>()
    clusters.forEach((c, i) => {
        for (const id of c.ids) clusterOf.set(id, i)
    })
    const clusterMinX = (ids: string[]): number => {
        let m = Number.POSITIVE_INFINITY
        for (const id of ids) {
            const b = placed.get(id)
            if (b !== undefined && b.x < m) m = b.x
        }
        return m
    }
    const order = clusters
        .map((_, i) => i)
        .sort((a, b) => clusterMinX(clusters[a]?.ids ?? []) - clusterMinX(clusters[b]?.ids ?? []))

    for (let oi = 1; oi < order.length; oi += 1) {
        const ci = order[oi]
        if (ci === undefined) continue
        const cluster = clusters[ci]
        if (cluster === undefined) continue
        // Skip a lone root with no block-tree children: its x is anchored
        // by the recenter pass to a bridged-out child in another cluster
        // (the "second mother" case) or it's a childless standalone —
        // packing it left would pull it off its child / serve no purpose.
        if (!cluster.hasOwnChildren) continue
        const ids = cluster.ids
        let minSlack = Number.POSITIVE_INFINITY
        for (const id of ids) {
            const b = placed.get(id)
            if (b === undefined) continue
            // Rightmost edge of any non-cluster block left of `b` on its row.
            let maxRight = Number.NEGATIVE_INFINITY
            for (const other of placed.values()) {
                if (clusterOf.get(other.id) === ci) continue
                if (other.y !== b.y) continue
                if (other.x >= b.x) continue
                const r = other.x + other.pixelWidth
                if (r > maxRight) maxRight = r
            }
            const slack = maxRight === Number.NEGATIVE_INFINITY ? b.x : b.x - (maxRight + CLUSTER_GAP)
            if (slack < minSlack) minSlack = slack
        }
        if (Number.isFinite(minSlack) && minSlack > 0.5) {
            for (const id of ids) {
                const b = placed.get(id)
                if (b !== undefined) placed.set(id, { ...b, x: b.x - minSlack })
            }
        }
    }
}
