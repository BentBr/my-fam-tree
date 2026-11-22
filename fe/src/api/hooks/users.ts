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
        onSuccess: (vars, profile) => {
            // Profile.locale on the wire is `string`; narrow before
            // forwarding into the two stores that require the literal union.
            if (profile.locale === 'en' || profile.locale === 'de') {
                locale.set(profile.locale)
                auth.patchUser({ displayName: profile.display_name, locale: profile.locale })
            } else {
                auth.patchUser({ displayName: profile.display_name })
            }
            // The locale is baked into the access JWT at mint time; this PATCH
            // only updates the DB row, so `/auth/me` (which echoes the token)
            // would keep returning the OLD locale on the next hydrate — the
            // choice would silently revert on reload until the user logs out.
            // Re-mint the session token from the now-updated DB row so the
            // preference persists. Best-effort: the in-memory + localStorage
            // state is already correct for this session, so a failed re-mint
            // must not surface as an unhandled rejection (it would crash the
            // turn under vitest's unhandled-error policy and is pointless noise
            // in prod — the next real refresh/login re-syncs the token).
            if (vars.locale !== undefined) {
                void auth.refresh().catch(() => {})
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
        mutationFn: () => expectOk(client.DELETE('/api/v1/users/me/avatar')),
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
