import { useQuery } from '@tanstack/vue-query'

import { client } from '../client'

export function useHealth() {
    return useQuery({
        queryKey: ['health'],
        queryFn: async () => {
            // The shared `errorTranslator` middleware in `client.ts` already converts
            // RFC 7807 errors into `ApiClientError` throws. We just unwrap success.
            const { data, error } = await client.GET('/api/v1/health')
            if (error !== undefined) throw error
            return data
        },
        retry: 0,
    })
}
