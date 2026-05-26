import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed } from 'vue'

import { i18n } from '@/i18n'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'

/**
 * Pending owner-transfer status as returned by `GET /transfer-owner` and
 * `POST /transfer-owner[/confirm]`. Mirrors backend `TransferStatus`. The
 * GET endpoint returns `null` when there's no pending transfer.
 */
export interface TransferStatusRow {
    id: string
    from_user_id: string
    to_user_id: string
    from_confirmed: boolean
    to_confirmed: boolean
    expires_at: string
}

/**
 * `useOwnerTransfer` — read the active pending transfer (admin/owner).
 * Disabled until an active family is selected. Resolves to `null` when
 * no transfer is pending.
 */
export function useOwnerTransfer() {
    const family = useActiveFamilyStore()
    return useQuery({
        queryKey: computed(() => ['owner-transfer', family.activeFamilyId] as const),
        enabled: computed(() => family.activeFamilyId !== null),
        queryFn: async (): Promise<TransferStatusRow | null> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useOwnerTransfer: no active family')
            const { data, error } = await client.GET('/api/v1/families/{family_id}/transfer-owner', {
                params: { path: { family_id: familyId } },
            })
            if (error !== undefined) throw error
            if (data === undefined) {
                throw new Error('empty response from GET /families/{id}/transfer-owner')
            }
            return (data.data as TransferStatusRow | null) ?? null
        },
    })
}

/**
 * `useBeginOwnerTransfer` — POST `/transfer-owner` with the target's
 * `user_id`. Owner-only on the backend. Invalidates the `owner-transfer`
 * + `members` caches on success so the banner + table re-render
 * immediately.
 */
export function useBeginOwnerTransfer() {
    const qc = useQueryClient()
    const ui = useUiStore()
    const family = useActiveFamilyStore()
    return useMutation({
        mutationFn: async (toUserId: string): Promise<TransferStatusRow> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useBeginOwnerTransfer: no active family')
            const { data, error } = await client.POST('/api/v1/families/{family_id}/transfer-owner', {
                params: { path: { family_id: familyId } },
                body: { to_user_id: toUserId },
            })
            if (error !== undefined) throw error
            if (data === undefined) {
                throw new Error('empty response from POST /families/{id}/transfer-owner')
            }
            return data.data as TransferStatusRow
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['owner-transfer'] })
            void qc.invalidateQueries({ queryKey: ['members'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.transfer_started') })
        },
    })
}

/**
 * `useConfirmOwnerTransfer` — POST `/transfer-owner/confirm` with the
 * token from the email link. Token-bearing: the backend derives the
 * acting side from the token-hash match. Returns the updated transfer
 * row so the caller can branch on whether BOTH sides are now confirmed.
 */
export function useConfirmOwnerTransfer() {
    const qc = useQueryClient()
    const ui = useUiStore()
    const family = useActiveFamilyStore()
    return useMutation({
        mutationFn: async (token: string): Promise<TransferStatusRow> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useConfirmOwnerTransfer: no active family')
            const { data, error } = await client.POST('/api/v1/families/{family_id}/transfer-owner/confirm', {
                params: { path: { family_id: familyId } },
                body: { token },
            })
            if (error !== undefined) throw error
            if (data === undefined) {
                throw new Error('empty response from POST /transfer-owner/confirm')
            }
            return data.data as TransferStatusRow
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['owner-transfer'] })
            void qc.invalidateQueries({ queryKey: ['members'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.transfer_confirmed') })
        },
    })
}

/**
 * `useCancelOwnerTransfer` — DELETE the active pending transfer. Owner
 * only on the backend.
 */
export function useCancelOwnerTransfer() {
    const qc = useQueryClient()
    const ui = useUiStore()
    const family = useActiveFamilyStore()
    return useMutation({
        mutationFn: async (): Promise<void> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useCancelOwnerTransfer: no active family')
            const { error } = await client.DELETE('/api/v1/families/{family_id}/transfer-owner', {
                params: { path: { family_id: familyId } },
            })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['owner-transfer'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.transfer_cancelled') })
        },
    })
}
