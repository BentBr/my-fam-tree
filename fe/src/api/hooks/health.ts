import { useQuery } from '@tanstack/vue-query'

import { client } from '../client'
import type { components } from '../schema'

/**
 * Extended health envelope: the BE body plus a FE-measured
 * `network_round_trip_ms`. The BE's `server_duration_ms` only covers
 * IN-handler work — TLS handshake, Nginx, geographic distance, and
 * actix middleware before the handler are all outside its window. The
 * FE timer wraps the outgoing fetch + the incoming parse so the
 * difference between the two numbers is exactly the network +
 * proxy overhead the user is paying.
 *
 * Type intersection (not `extends`) because the parent is a generated
 * schema type whose `data` field shape — `components.schemas.Health` —
 * doesn't survive a plain interface-extends across `openapi-typescript`
 * regeneration.
 */
export type HealthEnvelope = components['schemas']['HealthResponseBody'] & {
    /** Wall-clock fetch round-trip measured in the browser (ms). */
    network_round_trip_ms: number
}

export function useHealth() {
    return useQuery({
        queryKey: ['health'],
        queryFn: async (): Promise<HealthEnvelope> => {
            // NB: this query deliberately does NOT use the shared `unwrap()`
            // helper. `unwrap` returns the inner `.data`, but HealthView reads
            // the full envelope (`data.value?.data.version` AND
            // `data.value?.meta?.request_id`), so we keep returning the whole
            // body. The `errorTranslator` middleware already throws on RFC 7807
            // errors; the `{ error }` re-throw covers non-problem failures.
            //
            // `performance.now()` brackets the fetch so we capture the
            // full FE-perceived round-trip: TLS, Nginx, geographic
            // distance, server-side work, and the JSON parse on the
            // way back. The BE's `server_duration_ms` only sees the
            // IN-handler slice; subtracting one from the other tells
            // the user how much overhead lives "between the wires".
            const t0 = performance.now()
            const { data, error } = await client.GET('/api/v1/health')
            const networkMs = performance.now() - t0
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty health response')
            return { ...data, network_round_trip_ms: networkMs }
        },
        retry: 0,
    })
}
