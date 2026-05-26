import { useMutation } from '@tanstack/vue-query'

import { useAuthStore } from '@/stores/auth'

import { client } from '../client'
import { expectOk, unwrap } from '../request'

export function useRequestMagicLink() {
    // The "sent" envelope is ignored by callers (LoginView just shows the
    // check-inbox state regardless), so `expectOk` is the right shape —
    // it throws on error and resolves to void otherwise.
    return useMutation({
        mutationFn: (email: string) => expectOk(client.POST('/api/v1/auth/magic-link', { body: { email } })),
    })
}

export function useConsumeMagicLink() {
    const auth = useAuthStore()
    // Kept on raw `useMutation`: the consume response hydrates the auth
    // store, which must happen inside the mutation (not via a toast/
    // invalidate), so `useApiMutation` isn't the right fit. The network
    // call still routes through `unwrap()`.
    return useMutation({
        mutationFn: async (token: string) => {
            const claims = await unwrap(client.POST('/api/v1/auth/consume', { body: { token } }))
            auth.applyClaimsPayload(claims)
            return claims
        },
    })
}
