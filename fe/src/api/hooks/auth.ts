import { useMutation } from '@tanstack/vue-query'

import { useAuthStore } from '@/stores/auth'

import { client } from '../client'

export function useRequestMagicLink() {
    return useMutation({
        mutationFn: async (email: string) => {
            const { data, error } = await client.POST('/api/v1/auth/magic-link', {
                body: { email },
            })
            if (error !== undefined) throw error
            return data
        },
    })
}

export function useConsumeMagicLink() {
    const auth = useAuthStore()
    return useMutation({
        mutationFn: async (token: string) => {
            const { data, error } = await client.POST('/api/v1/auth/consume', {
                body: { token },
            })
            if (error !== undefined) throw error
            if (data !== undefined) {
                auth.applyClaimsPayload(data.data)
            }
            return data
        },
    })
}
