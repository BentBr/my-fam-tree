import { useQuery } from '@tanstack/vue-query'
import { type Ref } from 'vue'

import { client } from '../client'

/** One of `all`, `birthday`, `anniversary` (the route is forgiving on unknown). */
export type UpcomingFilter = 'all' | 'birthday' | 'anniversary'

/**
 * GET `/api/v1/upcoming` with a filter. The filter is reactive — the
 * query key includes it, so toggling causes a refetch keyed per filter
 * value. The `all` filter is passed without a query parameter to keep
 * the URL minimal; the backend's default is `all`.
 */
export function useUpcoming(filter: Ref<UpcomingFilter>) {
    return useQuery({
        queryKey: ['upcoming', filter] as const,
        queryFn: async () => {
            const f = filter.value
            // The backend treats a missing `filter` as `all`, so we
            // omit the query parameter entirely when filter is `all`
            // to keep the URL minimal. openapi-fetch's typed `query`
            // object disallows `undefined` under
            // `exactOptionalPropertyTypes`, so we split the call.
            const { data, error } =
                f === 'all'
                    ? await client.GET('/api/v1/upcoming')
                    : await client.GET('/api/v1/upcoming', { params: { query: { filter: f } } })
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from /upcoming')
            return data.data
        },
        // Keep previous data visible while a filter toggle re-fetches so the
        // toolbar doesn't flicker between an empty list and the new one.
        placeholderData: (prev) => prev,
    })
}
