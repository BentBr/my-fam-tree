import { useQuery } from '@tanstack/vue-query'
import { computed, type MaybeRefOrGetter, toValue } from 'vue'

import { client } from '../client'
import { expectOk, unwrap, useApiMutation } from '../request'

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
            const payload = await unwrap(client.GET('/api/v1/persons/{id}/contacts', { params: { path: { id: pid } } }))
            return payload.contacts
        },
    })
}

export function useCreateContact(personId: string) {
    return useApiMutation({
        mutationFn: (input: ContactInput) =>
            unwrap(
                client.POST('/api/v1/persons/{id}/contacts', {
                    params: { path: { id: personId } },
                    body: input as never,
                }),
            ),
        success: 'toasts.contact_created',
        invalidate: () => [['contacts', personId]],
    })
}

export function useUpdateContact(personId: string) {
    return useApiMutation({
        mutationFn: (vars: { id: string; input: ContactInput }) =>
            unwrap(
                client.PATCH('/api/v1/contacts/{id}', { params: { path: { id: vars.id } }, body: vars.input as never }),
            ),
        success: 'toasts.contact_updated',
        invalidate: () => [['contacts', personId]],
    })
}

export function useDeleteContact(personId: string) {
    return useApiMutation({
        mutationFn: (id: string) => expectOk(client.DELETE('/api/v1/contacts/{id}', { params: { path: { id } } })),
        success: 'toasts.contact_deleted',
        invalidate: () => [['contacts', personId]],
    })
}
