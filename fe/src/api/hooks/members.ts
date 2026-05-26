import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'

import { useActiveFamilyStore } from '@/stores/activeFamily'

import { client } from '../client'
import { expectOk, unwrap, useApiMutation } from '../request'

/**
 * Member row as returned by `GET /families/{id}/members`. Mirrors the
 * backend `MemberDto` plus the wrapping `{ data: [...] }` shape; we
 * unwrap the inner list here so the FE always works with `MemberRow[]`.
 */
export interface MemberRow {
    user_id: string
    email: string
    display_name: string
    role: 'user' | 'admin' | 'owner'
    joined_at: string
}

/**
 * `useMembers` — list every member of the active family. Disabled
 * until an active family is selected (otherwise the BE rejects with a
 * missing `X-Family-Id` validation error). Re-queries when the active
 * family changes.
 */
export function useMembers() {
    const family = useActiveFamilyStore()
    return useQuery({
        queryKey: computed(() => ['members', family.activeFamilyId] as const),
        enabled: computed(() => family.activeFamilyId !== null),
        queryFn: async (): Promise<MemberRow[]> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useMembers: no active family')
            // `MembersList` is `{ data: MemberDto[] }`; the envelope wraps it
            // again, so `unwrap` peels one layer and `.data` peels the inner.
            const list = await unwrap(
                client.GET('/api/v1/families/{family_id}/members', { params: { path: { family_id: familyId } } }),
            )
            return list.data as unknown as MemberRow[]
        },
    })
}

/**
 * `useSetRole` — PATCH a single member's role. The matrix gate lives
 * on the backend; the FE filters which buttons render based on the
 * same matrix to keep the UX honest, but the BE re-validates every
 * call. Invalidates the `members` cache on success.
 */
export function useSetRole() {
    const family = useActiveFamilyStore()
    return useApiMutation({
        mutationFn: (vars: { userId: string; role: 'user' | 'admin' }) => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useSetRole: no active family')
            return unwrap(
                client.PATCH('/api/v1/families/{family_id}/members/{user_id}', {
                    params: { path: { family_id: familyId, user_id: vars.userId } },
                    body: { role: vars.role },
                }),
            )
        },
        success: 'toasts.member_updated',
        invalidate: () => [['members']],
    })
}

/**
 * `useRevokeMember` — DELETE a member's row. Same matrix gating as
 * `useSetRole`.
 */
export function useRevokeMember() {
    const family = useActiveFamilyStore()
    return useApiMutation({
        mutationFn: (userId: string) => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useRevokeMember: no active family')
            return expectOk(
                client.DELETE('/api/v1/families/{family_id}/members/{user_id}', {
                    params: { path: { family_id: familyId, user_id: userId } },
                }),
            )
        },
        success: 'toasts.member_revoked',
        invalidate: () => [['members']],
    })
}
