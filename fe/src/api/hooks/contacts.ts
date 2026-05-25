import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, type MaybeRefOrGetter, toValue } from 'vue'

import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'

export type ContactKind = 'email' | 'phone' | 'address' | 'url' | 'other'
export type ContactVisibility = 'family' | 'admins_only'

export interface ContactInput {
    kind: ContactKind
    label?: string
    value: unknown
    visibility?: ContactVisibility
}

/**
 * `useContacts(personId)` — lists every contact row for the person.
 * The backend filters `admins_only` rows out automatically for `user`
 * role members.
 *
 * Accepts either a plain string (back-compat with one-shot mounts) or
 * a Vue ref / getter so the drawer can swap targets and the query
 * key follows the active person without remounting the component.
 */
export function useContacts(personId: MaybeRefOrGetter<string>) {
    const id = computed(() => toValue(personId))
    return useQuery({
        queryKey: ['contacts', id] as const,
        enabled: computed(() => id.value !== ''),
        queryFn: async () => {
            const pid = id.value
            if (pid === '') throw new Error('useContacts: empty personId')
            const { data, error } = await client.GET('/api/v1/persons/{id}/contacts', {
                params: { path: { id: pid } },
            })
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from GET /persons/{id}/contacts')
            return data.data.contacts
        },
    })
}

export function useCreateContact(personId: string) {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (input: ContactInput) => {
            const { data, error } = await client.POST('/api/v1/persons/{id}/contacts', {
                params: { path: { id: personId } },
                body: input as never,
            })
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from POST /persons/{id}/contacts')
            return data.data
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['contacts', personId] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.contact_created') })
        },
    })
}

export function useUpdateContact(personId: string) {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (vars: { id: string; input: ContactInput }) => {
            const { data, error } = await client.PATCH('/api/v1/contacts/{id}', {
                params: { path: { id: vars.id } },
                body: vars.input as never,
            })
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from PATCH /contacts/{id}')
            return data.data
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['contacts', personId] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.contact_updated') })
        },
    })
}

export function useDeleteContact(personId: string) {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (id: string) => {
            const { error } = await client.DELETE('/api/v1/contacts/{id}', {
                params: { path: { id } },
            })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['contacts', personId] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.contact_deleted') })
        },
    })
}
