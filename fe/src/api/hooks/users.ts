import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'

import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'

import { client } from '../client'
import { expectOk, unwrap, useApiMutation } from '../request'

interface UpdateMeBody {
    display_name?: string
    locale?: 'en' | 'de'
}

export function useMe() {
    const auth = useAuthStore()
    return useQuery({
        queryKey: ['users', 'me'],
        // `/users/me` requires the access cookie — gate the query on the auth
        // store's resolved-and-authenticated state so the AppBar (which
        // mounts on the sign-in page too) doesn't fire before the cookie
        // exists, get a clean 401, and trigger the FE's `session_expired`
        // toast on every page-load before login.
        enabled: computed(() => auth.status === 'authenticated'),
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

/**
 * Upload the caller's avatar. Mirrors `useSetPersonPhoto` — FormData body
 * with a single `file` field, multipart boundary set by fetch from the
 * FormData instance. Invalidates `['users','me']` so every render of the
 * profile + nav menu sees the new `avatar_url` immediately.
 */
export function useSetMyAvatar() {
    return useApiMutation({
        mutationFn: (file: File) => {
            const fd = new FormData()
            fd.append('file', file)
            return unwrap(
                client.POST('/api/v1/users/me/avatar', {
                    body: fd as unknown as string,
                    bodySerializer: (b: unknown) => b as BodyInit,
                }),
            )
        },
        success: 'toasts.user_avatar_set',
        // BE broadcasts the new avatar to every linked person across every
        // family — invalidate the trees + person caches so the tree
        // bubbles + person sidebar pick up the photo without a manual
        // refresh.
        invalidate: () => [['users', 'me'], ['tree'], ['persons'], ['person']],
    })
}

export function useClearMyAvatar() {
    return useApiMutation({
        // useApiMutation's TVars infers from mutationFn — accept and ignore
        // a void arg so call sites can still `mutateAsync()` cleanly via
        // a no-arg call when the type allows it.
        mutationFn: (_: void) => expectOk(client.DELETE('/api/v1/users/me/avatar')),
        success: 'toasts.user_avatar_cleared',
        // Same fan-out as the set path — the BE clears every linked
        // person's photo too.
        invalidate: () => [['users', 'me'], ['tree'], ['persons'], ['person']],
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
