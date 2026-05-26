import { useQuery } from '@tanstack/vue-query'

import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'

import { client } from '../client'
import { unwrap, useApiMutation } from '../request'

interface UpdateMeBody {
    display_name?: string
    locale?: 'en' | 'de'
}

export function useMe() {
    return useQuery({
        queryKey: ['users', 'me'],
        // Returns the inner profile (`unwrap` peels the envelope), so call
        // sites read `me.data.value?.display_name` directly.
        queryFn: () => unwrap(client.GET('/api/v1/users/me')),
    })
}

export function useUpdateMe() {
    const auth = useAuthStore()
    const locale = useLocaleStore()
    return useApiMutation({
        mutationFn: (body: UpdateMeBody) => unwrap(client.PATCH('/api/v1/users/me', { body })),
        success: 'toasts.profile_saved',
        invalidate: () => [['users', 'me']],
        onSuccess: (_vars, profile) => {
            // Profile.locale on the wire is `string`; narrow before
            // forwarding into the two stores that require the literal union.
            if (profile.locale === 'en' || profile.locale === 'de') {
                locale.set(profile.locale)
                auth.patchUser({ displayName: profile.display_name, locale: profile.locale })
            } else {
                auth.patchUser({ displayName: profile.display_name })
            }
        },
    })
}

export function useRequestEmailChange() {
    return useApiMutation({
        mutationFn: (newEmail: string) =>
            unwrap(client.POST('/api/v1/users/me/email-change', { body: { new_email: newEmail } })),
        success: 'toasts.email_change_sent',
    })
}

export function useConfirmEmailChange() {
    const auth = useAuthStore()
    return useApiMutation({
        mutationFn: (token: string) =>
            unwrap(client.POST('/api/v1/users/me/email-change/confirm', { body: { token } })),
        success: 'toasts.email_changed',
        invalidate: () => [['users', 'me']],
        onSuccess: (_vars, profile) => {
            // Keep the cached email in the auth store in sync — the confirm
            // endpoint may also have re-issued the access cookie, but the
            // store only sees that on the next hydrate.
            auth.patchUser({ displayName: profile.display_name, email: profile.email })
        },
    })
}
