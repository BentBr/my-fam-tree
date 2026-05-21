import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'

import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'

export interface PersonInput {
    given_name: string
    family_name?: string
    name_at_birth?: string
    nickname?: string
    gender?: string
    birth_date?: string | null
    birth_place?: string
    death_date?: string | null
    notes?: string
    linked_user_id?: string | null
}

export interface PersonUpdateInput {
    given_name?: string | null
    family_name?: string | null
    name_at_birth?: string | null
    nickname?: string | null
    gender?: string | null
    birth_date?: string | null
    birth_place?: string | null
    death_date?: string | null
    notes?: string | null
    linked_user_id?: string | null
}

// `PersonsListResponseBody` on the wire is `{ data: PersonView[], meta? }` — we
// return `data.data` (the `PersonView[]` array) directly so call sites don't
// have to peel an extra wrapping layer that exists purely for response metadata.
//
// openapi-fetch types `data` as `T | undefined` (it's defined when `error` is),
// so we re-check after the `error` guard. The `Error('empty response')` paths
// only fire on protocol-level surprises like an empty 204 — they keep the
// types honest under `noUncheckedIndexedAccess`.
export function useListPersons() {
    return useQuery({
        queryKey: ['persons'],
        queryFn: async () => {
            const { data, error } = await client.GET('/api/v1/persons')
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from /persons')
            return data.data
        },
    })
}

export function useCreatePerson() {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (input: PersonInput) => {
            const { data, error } = await client.POST('/api/v1/persons', { body: input })
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from POST /persons')
            return data.data
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['persons'] })
            void qc.invalidateQueries({ queryKey: ['tree'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.person_created') })
        },
    })
}

export function useUpdatePerson() {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (vars: { id: string; input: PersonUpdateInput }) => {
            const { data, error } = await client.PATCH('/api/v1/persons/{id}', {
                params: { path: { id: vars.id } },
                body: vars.input,
            })
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from PATCH /persons/{id}')
            return data.data
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['persons'] })
            void qc.invalidateQueries({ queryKey: ['tree'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.person_updated') })
        },
    })
}

export function useDeletePerson() {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (id: string) => {
            const { error } = await client.DELETE('/api/v1/persons/{id}', {
                params: { path: { id } },
            })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['persons'] })
            void qc.invalidateQueries({ queryKey: ['tree'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.person_deleted') })
        },
    })
}
