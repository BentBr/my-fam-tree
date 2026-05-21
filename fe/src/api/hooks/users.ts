import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'

import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'

import { client } from '../client'

interface UpdateMeBody {
    display_name?: string
    locale?: 'en' | 'de'
}

export function useMe() {
    return useQuery({
        queryKey: ['users', 'me'],
        queryFn: async () => {
            const { data, error } = await client.GET('/api/v1/users/me')
            if (error !== undefined) throw error
            return data
        },
    })
}

export function useUpdateMe() {
    const auth = useAuthStore()
    const locale = useLocaleStore()
    const qc = useQueryClient()
    return useMutation({
        mutationFn: async (body: UpdateMeBody) => {
            const { data, error } = await client.PATCH('/api/v1/users/me', { body })
            if (error !== undefined) throw error
            return data
        },
        onSuccess: (data) => {
            qc.invalidateQueries({ queryKey: ['users', 'me'] })
            if (data === undefined) return
            const profile = data.data
            // Profile.locale on the wire is `string`; narrow before forwarding
            // into the two stores that require the literal union.
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
    return useMutation({
        mutationFn: async (newEmail: string) => {
            const { data, error } = await client.POST('/api/v1/users/me/email-change', {
                body: { new_email: newEmail },
            })
            if (error !== undefined) throw error
            return data
        },
    })
}

export function useConfirmEmailChange() {
    const auth = useAuthStore()
    const qc = useQueryClient()
    return useMutation({
        mutationFn: async (token: string) => {
            const { data, error } = await client.POST('/api/v1/users/me/email-change/confirm', {
                body: { token },
            })
            if (error !== undefined) throw error
            return data
        },
        onSuccess: (data) => {
            qc.invalidateQueries({ queryKey: ['users', 'me'] })
            if (data === undefined) return
            // Keep the cached email in the auth store in sync — the confirm
            // endpoint may also have re-issued the access cookie, but the
            // store only sees that on the next hydrate.
            auth.patchUser({ displayName: data.data.display_name, email: data.data.email })
        },
    })
}
