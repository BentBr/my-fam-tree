import { useQuery } from '@tanstack/vue-query'
import { computed, type MaybeRefOrGetter, toValue } from 'vue'

import { useActiveFamilyStore } from '@/stores/activeFamily'

import { client } from '../client'
import { unwrap } from '../request'

/**
 * Filter shape for `useAuditList` — mirrors the backend's `AuditQuery`
 * parameters but uses camelCase to match the rest of the FE.
 *
 * `from` / `to` are ISO-8601 datetimes (inclusive bounds). `page` is
 * 1-based. `pageSize` must be one of 50 / 100 / 200 / 500 — anything
 * else falls back to 50 server-side.
 */
export interface AuditFilter {
    page?: number
    pageSize?: number
    from?: string
    to?: string
    action?: string
    entityKind?: string
    actorUserId?: string
}

/**
 * `GET /api/v1/families/{id}/audit`. Reactively re-fetches when the
 * filter changes or the active family changes. Disabled until an
 * active family is selected — otherwise the BE rejects the request
 * with a missing `X-Family-Id` validation error.
 *
 * Returns the unwrapped `AuditPage` (the full envelope's `data` field).
 */
export function useAuditList(filter: MaybeRefOrGetter<AuditFilter>) {
    const family = useActiveFamilyStore()
    return useQuery({
        queryKey: computed(() => ['audit', family.activeFamilyId, toValue(filter)] as const),
        enabled: computed(() => family.activeFamilyId !== null),
        queryFn: async () => {
            const f = toValue(filter)
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useAuditList: no active family')
            // openapi-fetch's typed `query` rejects `undefined` under
            // `exactOptionalPropertyTypes`; build the object with only
            // the keys the caller actually set.
            const query: Record<string, number | string> = {}
            if (f.page !== undefined) query['page'] = f.page
            if (f.pageSize !== undefined) query['page_size'] = f.pageSize
            if (f.from !== undefined) query['from'] = f.from
            if (f.to !== undefined) query['to'] = f.to
            if (f.action !== undefined) query['action'] = f.action
            if (f.entityKind !== undefined) query['entity_kind'] = f.entityKind
            if (f.actorUserId !== undefined) query['actor_user_id'] = f.actorUserId
            return unwrap(
                client.GET('/api/v1/families/{family_id}/audit', {
                    params: {
                        path: { family_id: familyId },
                        query,
                    },
                }),
            )
        },
        // Keep previous data visible while filters change so the table
        // doesn't flash between "empty" and the new page.
        placeholderData: (prev) => prev,
    })
}
