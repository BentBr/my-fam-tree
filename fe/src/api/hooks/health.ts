import { useQuery } from '@tanstack/vue-query'

import { client } from '../client'

export function useHealth() {
    return useQuery({
        queryKey: ['health'],
        queryFn: async () => {
            // NB: this query deliberately does NOT use the shared `unwrap()`
            // helper. `unwrap` returns the inner `.data`, but HealthView reads
            // the full envelope (`data.value?.data.version` AND
            // `data.value?.meta?.request_id`), so we keep returning the whole
            // body. The `errorTranslator` middleware already throws on RFC 7807
            // errors; the `{ error }` re-throw covers non-problem failures.
            const { data, error } = await client.GET('/api/v1/health')
            if (error !== undefined) throw error
            return data
        },
        retry: 0,
    })
}
