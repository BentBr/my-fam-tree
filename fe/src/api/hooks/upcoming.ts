import { useQuery } from '@tanstack/vue-query'
import { computed, type Ref } from 'vue'

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
            const query: Record<string, string> | undefined = f === 'all' ? undefined : { filter: f }
            const { data, error } = await client.GET('/api/v1/upcoming', {
                params: { query },
            })
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from /upcoming')
            return data.data
        },
        // Keep previous data visible while a filter toggle re-fetches so the
        // toolbar doesn't flicker between an empty list and the new one.
        placeholderData: (prev) => prev,
    })
}

/** Reactive helper to format an `UpcomingFilter` as the i18n suffix on `upcoming.kinds.*`. */
export function kindLabelKey(kind: string): string {
    return computed(() => `upcoming.kinds.${kind}`).value
}
