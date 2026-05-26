import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'

import { useActiveFamilyStore } from '@/stores/activeFamily'

import { client } from '../client'
import { expectOk, unwrap, useApiMutation } from '../request'

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
            const status = await unwrap(
                client.GET('/api/v1/families/{family_id}/transfer-owner', {
                    params: { path: { family_id: familyId } },
                }),
            )
            return (status as TransferStatusRow | null) ?? null
        },
    })
}

/**
 * `useBeginOwnerTransfer` — POST `/transfer-owner` with the target's
 * `user_id`. Owner-only on the backend. Invalidates the `owner-transfer`
 * + `members` caches on success so the banner + table re-render.
 */
export function useBeginOwnerTransfer() {
    const family = useActiveFamilyStore()
    return useApiMutation({
        mutationFn: (toUserId: string): Promise<TransferStatusRow> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useBeginOwnerTransfer: no active family')
            return unwrap(
                client.POST('/api/v1/families/{family_id}/transfer-owner', {
                    params: { path: { family_id: familyId } },
                    body: { to_user_id: toUserId },
                }),
            ) as Promise<TransferStatusRow>
        },
        success: 'toasts.transfer_started',
        invalidate: () => [['owner-transfer'], ['members']],
    })
}

/**
 * `useConfirmOwnerTransfer` — POST `/transfer-owner/confirm` with the
 * token from the email link. Token-bearing: the backend derives the
 * acting side from the token-hash match. Returns the updated transfer
 * row so the caller can branch on whether BOTH sides are now confirmed.
 */
export function useConfirmOwnerTransfer() {
    const family = useActiveFamilyStore()
    return useApiMutation({
        mutationFn: (token: string): Promise<TransferStatusRow> => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useConfirmOwnerTransfer: no active family')
            return unwrap(
                client.POST('/api/v1/families/{family_id}/transfer-owner/confirm', {
                    params: { path: { family_id: familyId } },
                    body: { token },
                }),
            ) as Promise<TransferStatusRow>
        },
        success: 'toasts.transfer_confirmed',
        invalidate: () => [['owner-transfer'], ['members']],
    })
}

/**
 * `useCancelOwnerTransfer` — DELETE the active pending transfer. Owner
 * only on the backend.
 */
export function useCancelOwnerTransfer() {
    const family = useActiveFamilyStore()
    return useApiMutation<void, void>({
        mutationFn: () => {
            const familyId = family.activeFamilyId
            if (familyId === null) throw new Error('useCancelOwnerTransfer: no active family')
            return expectOk(
                client.DELETE('/api/v1/families/{family_id}/transfer-owner', {
                    params: { path: { family_id: familyId } },
                }),
            )
        },
        success: 'toasts.transfer_cancelled',
        invalidate: () => [['owner-transfer']],
    })
}
