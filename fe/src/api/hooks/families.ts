import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'
import { unwrap } from '../request'

// Kept on raw `useMutation` (not `useApiMutation`): both hooks hydrate the
// auth store from the response and invalidate ALL queries (not a fixed key
// list), neither of which the wrapper models. The network call still routes
// through `unwrap()` so the error/empty handling is identical to every other
// hook. `unwrap` returns the inner `{ family, claims }` — call sites read
// `res.family.id`.
export function useCreateFamily() {
    const auth = useAuthStore()
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (name: string) => {
            const payload = await unwrap(client.POST('/api/v1/families', { body: { name } }))
            auth.applyClaimsPayload(payload.claims)
            return payload
        },
        onSuccess: () => {
            qc.invalidateQueries()
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.family_created') })
        },
    })
}

export function useAcceptInvite() {
    const auth = useAuthStore()
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (token: string) => {
            const payload = await unwrap(client.POST('/api/v1/invites/accept', { body: { token } }))
            auth.applyClaimsPayload(payload.claims)
            return payload
        },
        onSuccess: () => {
            qc.invalidateQueries()
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.invite_accepted') })
        },
    })
}
