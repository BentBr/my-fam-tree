import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'

import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'

// `TreePayloadResponseBody` is `{ data: TreePayload, meta? }`. We unwrap to the
// `TreePayload` (nodes + parent_edges + partner_edges) so the FE layout code
// can consume it directly.
export function useTree() {
    return useQuery({
        queryKey: ['tree'],
        queryFn: async () => {
            const { data, error } = await client.GET('/api/v1/relationships')
            if (error !== undefined) throw error
            if (data === undefined) throw new Error('empty response from /relationships')
            return data.data
        },
    })
}

export interface ParentLinkInput {
    child_id: string
    parent_id: string
    kind: string
    note?: string
}

// Parent-link add: no success toast. The new edge appearing in the tree IS
// the confirmation; the global error toast (queryClient.ts) still surfaces
// validation/conflict failures.
export function useAddParentLink() {
    const qc = useQueryClient()
    return useMutation({
        mutationFn: async (vars: ParentLinkInput) => {
            const { error } = await client.POST('/api/v1/parent-links', { body: vars })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['tree'] })
        },
    })
}

export interface PartnershipInput {
    partner_a_id: string
    partner_b_id: string
    kind: string
    started_on?: string | null
    ended_on?: string | null
    end_reason?: string | null
    note?: string
}

export function useCreatePartnership() {
    const qc = useQueryClient()
    return useMutation({
        mutationFn: async (vars: PartnershipInput) => {
            const { error } = await client.POST('/api/v1/partnerships', { body: vars })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['tree'] })
        },
    })
}

/**
 * PATCH `/api/v1/partnerships/{id}`. Every input field is optional; the
 * backend merges into the existing row and rejects a fully-empty body with
 * `validation.value_required`. The success toast wording matches the
 * existing `person_updated` cadence — we keep partnership feedback in the
 * generic "row updated" family rather than introducing a new key.
 */
export interface PartnershipUpdateInput {
    kind?: string | null
    started_on?: string | null
    ended_on?: string | null
    end_reason?: string | null
    note?: string | null
}

export function useUpdatePartnership() {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (vars: { id: string; input: PartnershipUpdateInput }) => {
            const { error } = await client.PATCH('/api/v1/partnerships/{id}', {
                params: { path: { id: vars.id } },
                body: vars.input,
            })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['tree'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.partnership_updated') })
        },
    })
}

export function useDeletePartnership() {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: async (id: string) => {
            const { error } = await client.DELETE('/api/v1/partnerships/{id}', {
                params: { path: { id } },
            })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['tree'] })
            ui.pushToast({ kind: 'success', message: i18n.global.t('toasts.partnership_deleted') })
        },
    })
}

/**
 * DELETE `/api/v1/parent-links/{child}/{parent}`. There is no PATCH endpoint
 * for parent links, so the FE pairs this with `useAddParentLink` to "change
 * kind" via delete-then-recreate. The mutation does NOT push its own toast
 * — the calling flow stitches DELETE+POST and pushes a single combined
 * toast at the end to avoid double notifications.
 */
export function useDeleteParentLink() {
    const qc = useQueryClient()
    return useMutation({
        mutationFn: async (vars: { child_id: string; parent_id: string }) => {
            const { error } = await client.DELETE('/api/v1/parent-links/{child}/{parent}', {
                params: { path: { child: vars.child_id, parent: vars.parent_id } },
            })
            if (error !== undefined) throw error
        },
        onSuccess: () => {
            void qc.invalidateQueries({ queryKey: ['tree'] })
        },
    })
}
