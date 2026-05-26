import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed } from 'vue'

import { i18n } from '@/i18n'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'

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
            const { data, error } = await client.GET('/api/v1/families/{id}/invites', {
                params: { path: { id: familyId } },
            })
            if (error !== undefined) throw error
            if (data === undefined) {
                throw new Error('empty response from GET /families/{id}/invites')
            }
            return data.data.data as unknown as InviteRow[]
        },
    })
}

/**
 * `useCreateInvite` — POST a new invite. Used by the PersonDetail CTA and
 * (potentially) the bare admin invite form. Invalidates the `invites`
 * cache on success so the admin pending-list refreshes. Toast string
 * lives in `toasts.invite_sent`.
 */
export function useCreateInvite() {
    const qc = useQueryClient()
    const ui = useUiStore()
    const family = useActiveFamilyStore()
    return useMutation({
        mutationFn: async (vars: { email: string; role: 'user' | 'admin'; personId?: string | null }) => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useCreateInvite: no active family')
            const { data, error } = await client.POST('/api/v1/families/{id}/invites', {
                params: { path: { id: familyId } },
                body: {
                    email: vars.email,
                    role: vars.role,
                    person_id: vars.personId ?? null,
                },
            })
            if (error !== undefined) throw error
            if (data === undefined) {
                throw new Error('empty response from POST /families/{id}/invites')
            }
            return data.data
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['invites'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.invite_sent') })
        },
    })
}

/**
 * `useCancelInvite` — DELETE a pending invite. Same admin/owner gate as
 * `useCreateInvite`. Returns `void` on success.
 */
export function useCancelInvite() {
    const qc = useQueryClient()
    const ui = useUiStore()
    const family = useActiveFamilyStore()
    return useMutation({
        mutationFn: async (inviteId: string): Promise<void> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useCancelInvite: no active family')
            const { error } = await client.DELETE('/api/v1/families/{id}/invites/{invite_id}', {
                params: { path: { id: familyId, invite_id: inviteId } },
            })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['invites'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.invite_cancelled') })
        },
    })
}
