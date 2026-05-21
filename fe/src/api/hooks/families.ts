import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'

import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'

export function useMyFamilies() {
    return useQuery({
        queryKey: ['families', 'me'],
        queryFn: async () => {
            const { data, error } = await client.GET('/api/v1/families/me')
            if (error !== undefined) throw error
            return data
        },
    })
}

export function useCreateFamily() {
    const auth = useAuthStore()
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (name: string) => {
            const { data, error } = await client.POST('/api/v1/families', { body: { name } })
            if (error !== undefined) throw error
            if (data !== undefined) {
                auth.applyClaimsPayload(data.data.claims)
            }
            return data
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
            const { data, error } = await client.POST('/api/v1/invites/accept', { body: { token } })
            if (error !== undefined) throw error
            if (data !== undefined) {
                auth.applyClaimsPayload(data.data.claims)
            }
            return data
        },
        onSuccess: () => {
            qc.invalidateQueries()
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.invite_accepted') })
        },
    })
}
