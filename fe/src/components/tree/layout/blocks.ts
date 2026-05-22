// Block construction + sort helpers for layout v2. A block is the placement
// unit on each row: either one person (singleton) or two same-row partners
// (couple) drawn side-by-side. This module does the partner-pairing pass
// and the sibling-ordering math; the recursive subtree placement lives in
// `./subtree.ts`.

import { birthSortKey } from './generations'
import { type BackendNode, type Block } from './types'

/**
 * Build the per-generation block list. For each row we walk the row members
 * in stable id order and pair anyone partnered to a same-row peer that hasn't
 * already been paired. Everyone else becomes a singleton block.
 */
export function buildBlocks(
    nodeIds: string[],
    generation: Map<string, number>,
    partnerOf: Map<string, Set<string>>,
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

    const blocks = new Map<number, Block[]>()
    for (const [g, ids] of byRow.entries()) {
        const consumed = new Set<string>()
        const list: Block[] = []
        for (const id of ids) {
            if (consumed.has(id)) continue
            const partners = partnerOf.get(id) ?? new Set<string>()
            // Pick the smallest-id same-row partner that hasn't been paired yet.
            let mate: string | null = null
            for (const p of partners) {
                if (consumed.has(p)) continue
                if ((generation.get(p) ?? -1) !== g) continue
                if (mate === null || p < mate) mate = p
            }
            if (mate !== null) {
                consumed.add(id)
                consumed.add(mate)
                // Left member is the smaller id for stable visuals.
                const left = id < mate ? id : mate
                const right = id < mate ? mate : id
                list.push({
                    id: `couple:${left}|${right}`,
                    members: [left, right],
                    y: 0, // filled in later
                    width: 2,
                })
            } else {
                consumed.add(id)
                list.push({
                    id: `single:${id}`,
                    members: [id],
                    y: 0,
                    width: 1,
                })
            }
        }
        blocks.set(g, list)
    }
    return blocks
}

/**
 * Choose a canonical parent block for each non-top block. A block hangs from
 * one parent block (the block that contains its canonical parent person);
 * extra parent edges still render as straight lines but don't influence
 * placement. Couples inherit the canonical parent of their LEFT member,
 * which keeps the tree shape predictable when both partners have known
 * ancestors.
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
