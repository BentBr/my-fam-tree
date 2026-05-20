import { useMutation } from '@tanstack/vue-query'

export interface ConsumeResult {
    user_id: string
    email: string
    locale: 'en' | 'de'
}

export function useRequestMagicLink() {
    return useMutation({
        mutationFn: async (email: string) => {
            // Phase 0d stub: real client.POST('/api/v1/auth/magic-link', ...) lands in Phase 1b.
            await new Promise((resolve) => setTimeout(resolve, 250))
            if (email.length === 0 || !email.includes('@')) {
                throw new Error('Invalid email address')
            }
        },
    })
}

export function useConsumeMagicLink() {
    return useMutation({
        mutationFn: async (token: string): Promise<ConsumeResult> => {
            // Phase 0d stub: real client.POST('/api/v1/auth/consume', ...) lands in Phase 1b.
            await new Promise((resolve) => setTimeout(resolve, 250))
            if (token.length === 0) {
                throw new Error('Missing token')
            }
            return { user_id: 'stub-user', email: 'stub@example.com', locale: 'en' }
        },
    })
}
