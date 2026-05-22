import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'

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
