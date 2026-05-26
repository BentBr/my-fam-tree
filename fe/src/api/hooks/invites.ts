import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'

import { useActiveFamilyStore } from '@/stores/activeFamily'

import { client } from '../client'
import { expectOk, unwrap, useApiMutation } from '../request'

/**
 * Pending invite row as returned by `GET /families/{id}/invites`. Mirrors
 * the backend `InviteDto` plus the wrapping `{ data: [...] }` shape; we
 * unwrap the inner list here so the FE always works with `InviteRow[]`.
 */
export interface InviteRow {
    id: string
    email: string
    role: 'user' | 'admin' | 'owner'
    person_id: string | null
    expires_at: string
    invited_by: string
}

/**
 * `useInvites` — list every pending invite for the active family. Disabled
 * until an active family is selected. Re-queries when the active family
 * changes.
 */
export function useInvites() {
    const family = useActiveFamilyStore()
    return useQuery({
        queryKey: computed(() => ['invites', family.activeFamilyId] as const),
        enabled: computed(() => family.activeFamilyId !== null),
        queryFn: async (): Promise<InviteRow[]> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useInvites: no active family')
            const list = await unwrap(
                client.GET('/api/v1/families/{id}/invites', { params: { path: { id: familyId } } }),
            )
            return list.data as unknown as InviteRow[]
        },
    })
}

/**
 * `useCreateInvite` — POST a new invite. Used by the PersonDetail CTA and
 * (potentially) the bare admin invite form. Invalidates the `invites`
 * cache on success so the admin pending-list refreshes.
 */
export function useCreateInvite() {
    const family = useActiveFamilyStore()
    return useApiMutation({
        mutationFn: (vars: { email: string; role: 'user' | 'admin'; personId?: string | null }) => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useCreateInvite: no active family')
            return unwrap(
                client.POST('/api/v1/families/{id}/invites', {
                    params: { path: { id: familyId } },
                    body: { email: vars.email, role: vars.role, person_id: vars.personId ?? null },
                }),
            )
        },
        success: 'toasts.invite_sent',
        invalidate: () => [['invites']],
    })
}

/**
 * `useCancelInvite` — DELETE a pending invite. Same admin/owner gate as
 * `useCreateInvite`.
 */
export function useCancelInvite() {
    const family = useActiveFamilyStore()
    return useApiMutation({
        mutationFn: (inviteId: string) => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useCancelInvite: no active family')
            return expectOk(
                client.DELETE('/api/v1/families/{id}/invites/{invite_id}', {
                    params: { path: { id: familyId, invite_id: inviteId } },
                }),
            )
        },
        success: 'toasts.invite_cancelled',
        invalidate: () => [['invites']],
    })
}
