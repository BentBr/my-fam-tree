import { useQuery } from '@tanstack/vue-query'
import { ref, type Ref } from 'vue'

import { client } from '../client'

/** One of `all`, `birthday`, `anniversary` (the route is forgiving on unknown). */
export type UpcomingFilter = 'all' | 'birthday' | 'anniversary'

// Shared "always false" ref for callers that don't pass the favourites
// gate. Hoisted so every default-arg invocation reuses the same ref
// (the query key compares by reference identity for refs).
const NEVER_FAVOURITES_ONLY: Ref<boolean> = ref(false)

/**
 * GET `/api/v1/upcoming` with a filter + optional favourites gate. The
 * filter is reactive — the query key includes both `filter` and
 * `favouritesOnly`, so toggling either causes a refetch keyed per
 * combined state. The `all` filter is passed without a query parameter
 * to keep the URL minimal; the backend's default is `all`.
 *
 * Favourites are per-user: the BE resolves `favourites_only=true`
 * against the signed-in caller's mark set, so two members of the same
 * family see independent results for the same toggle position. The
 * `favouritesOnly` ref is optional — callers that don't need the gate
 * (and existing tests) can omit it and get the unfiltered projection.
 */
export function useUpcoming(filter: Ref<UpcomingFilter>, favouritesOnly: Ref<boolean> = NEVER_FAVOURITES_ONLY) {
    return useQuery({
        queryKey: ['upcoming', filter, favouritesOnly] as const,
        queryFn: async () => {
            const f = filter.value
            const fav = favouritesOnly.value
            // The backend treats a missing `filter` as `all`. Build the
            // query object conditionally so openapi-fetch's typed `query`
            // doesn't see `undefined` properties — disallowed under
            // `exactOptionalPropertyTypes`.
            const query: { filter?: 'birthday' | 'anniversary'; favourites_only?: boolean } = {}
            if (f !== 'all') query.filter = f
            if (fav) query.favourites_only = true
            const { data, error } =
                Object.keys(query).length === 0
                    ? await client.GET('/api/v1/upcoming')
                    : await client.GET('/api/v1/upcoming', { params: { query } })
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from /upcoming')
            return data.data
        },
        // Keep previous data visible while a filter toggle re-fetches so the
        // toolbar doesn't flicker between an empty list and the new one.
        placeholderData: (prev) => prev,
    })
}
