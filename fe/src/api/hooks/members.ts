import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed } from 'vue'

import { i18n } from '@/i18n'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'

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
            const { data, error } = await client.GET('/api/v1/families/{family_id}/members', {
                params: { path: { family_id: familyId } },
            })
            if (error !== undefined) throw error
            if (data === undefined) {
                throw new Error('empty response from GET /families/{id}/members')
            }
            // Unwrap `{ data: { data: MemberDto[] } }` → `MemberRow[]`.
            return data.data.data as unknown as MemberRow[]
        },
    })
}

/**
 * `useSetRole` — PATCH a single member's role. The matrix gate lives
 * on the backend; the FE filters which buttons render based on the
 * same matrix to keep the UX honest, but the BE re-validates every
 * call. Invalidates the `members` cache on success so the table
 * re-renders with the fresh role chip.
 */
export function useSetRole() {
    const qc = useQueryClient()
    const ui = useUiStore()
    const family = useActiveFamilyStore()
    return useMutation({
        mutationFn: async (vars: { userId: string; role: 'user' | 'admin' }) => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useSetRole: no active family')
            const { data, error } = await client.PATCH('/api/v1/families/{family_id}/members/{user_id}', {
                params: { path: { family_id: familyId, user_id: vars.userId } },
                body: { role: vars.role },
            })
            if (error !== undefined) throw error
            if (data === undefined) {
                throw new Error('empty response from PATCH /families/{id}/members/{user_id}')
            }
            return data.data
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['members'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.member_updated') })
        },
    })
}

/**
 * `useRevokeMember` — DELETE a member's row. Same matrix gating as
 * `useSetRole`. Returns `void` on success.
 */
export function useRevokeMember() {
    const qc = useQueryClient()
    const ui = useUiStore()
    const family = useActiveFamilyStore()
    return useMutation({
        mutationFn: async (userId: string): Promise<void> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useRevokeMember: no active family')
            const { error } = await client.DELETE('/api/v1/families/{family_id}/members/{user_id}', {
                params: { path: { family_id: familyId, user_id: userId } },
            })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['members'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.member_revoked') })
        },
    })
}
