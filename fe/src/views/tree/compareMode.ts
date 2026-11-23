// Compare-mode state for the tree view. URL `?compare=a` / `?compare=a,b` is
// the source of truth for the selected pair, like `?center=` is for the
// drawer. Everything except `useCompareMode` is pure.

import { computed, ref, type ComputedRef, type Ref } from 'vue'
import { useRoute, type LocationQuery } from 'vue-router'

const UUID = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/

/**
 * Read `?compare=` by shape only: at most two distinct, well-formed ids,
 * lower-cased (the API serializes ids in lower case). Everything else is
 * dropped silently, like a bad `?center=`.
 */
export function parseCompareParam(raw: unknown): string[] {
    if (typeof raw !== 'string') return []
    const ids: string[] = []
    for (const part of raw.split(',')) {
        const id = part.toLowerCase()
        if (UUID.test(id) && !ids.includes(id) && ids.length < 2) ids.push(id)
    }
    return ids
}

/**
 * The ids that may be shown and asked about: those that are persons of the
 * tree on screen. Empty while that tree is unknown (`known === null`).
 */
export function knownPair(ids: readonly string[], known: ReadonlySet<string> | null): string[] {
    return known === null ? [] : ids.filter((id) => known.has(id))
}

/**
 * The pair after clicking `id`: a selected person is deselected (the survivor
 * moves to slot A), a free slot is filled, and with both slots taken B is
 * replaced.
 */
export function nextComparePair(current: readonly string[], id: string): string[] {
    if (current.includes(id)) return current.filter((x) => x !== id)
    const [first] = current
    return first === undefined ? [id] : [first, id]
}

export interface TreeQueryPatch {
    /** New `?center=`; null removes it; omitted leaves it alone. */
    center?: string | null
    /** New `?compare=` pair; empty removes it; omitted leaves it alone. */
    compare?: readonly string[]
}

/**
 * The tree view's query with `patch` applied. Callers pass ALL changes of one
 * interaction in a single patch: two back-to-back `router.replace` calls both
 * start from the same stale `route.query`, so the second would undo the first.
 */
export function withTreeQuery(query: LocationQuery, patch: TreeQueryPatch): LocationQuery {
    const next = { ...query }
    if (patch.center !== undefined) {
        if (patch.center === null) delete next['center']
        else next['center'] = patch.center
    }
    if (patch.compare !== undefined) {
        if (patch.compare.length === 0) delete next['compare']
        else next['compare'] = patch.compare.join(',')
    }
    return next
}

export interface CompareMode {
    /** Whether compare mode is on. Owned by the view's mode transitions. */
    active: Ref<boolean>
    /** Ids in `?compare=`, by shape only — may name persons that are not in the tree. */
    urlPair: ComputedRef<string[]>
    /** The usable selection: `urlPair` limited to persons of the tree on screen. */
    pair: ComputedRef<string[]>
    a: ComputedRef<string | null>
    b: ComputedRef<string | null>
}

/** `knownIds` returns the person ids of the tree on screen, or null while that is unknown. */
export function useCompareMode(knownIds: () => ReadonlySet<string> | null): CompareMode {
    const route = useRoute()
    const urlPair = computed<string[]>(() => parseCompareParam(route.query['compare']))
    const pair = computed<string[]>(() => knownPair(urlPair.value, knownIds()))
    return {
        active: ref(false),
        urlPair,
        pair,
        a: computed<string | null>(() => pair.value[0] ?? null),
        b: computed<string | null>(() => pair.value[1] ?? null),
    }
}
