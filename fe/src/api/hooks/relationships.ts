import { useQuery } from '@tanstack/vue-query'

import { client } from '../client'
import { expectOk, unwrap, useApiMutation } from '../request'

// `TreePayloadResponseBody` is `{ data: TreePayload, meta? }`. We unwrap to the
// `TreePayload` (nodes + parent_edges + partner_edges) so the FE layout code
// can consume it directly.
export function useTree() {
    return useQuery({
        queryKey: ['tree'],
        queryFn: () => unwrap(client.GET('/api/v1/relationships')),
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
    return useApiMutation({
        mutationFn: (vars: ParentLinkInput) => expectOk(client.POST('/api/v1/parent-links', { body: vars })),
        invalidate: () => [['tree']],
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
    return useApiMutation({
        mutationFn: (vars: PartnershipInput) => expectOk(client.POST('/api/v1/partnerships', { body: vars })),
        invalidate: () => [['tree']],
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
    return useApiMutation({
        mutationFn: (vars: { id: string; input: PartnershipUpdateInput }) =>
            expectOk(
                client.PATCH('/api/v1/partnerships/{id}', { params: { path: { id: vars.id } }, body: vars.input }),
            ),
        success: 'toasts.partnership_updated',
        invalidate: () => [['tree']],
    })
}

export function useDeletePartnership() {
    return useApiMutation({
        mutationFn: (id: string) => expectOk(client.DELETE('/api/v1/partnerships/{id}', { params: { path: { id } } })),
        success: 'toasts.partnership_deleted',
        invalidate: () => [['tree']],
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
    return useApiMutation({
        mutationFn: (vars: { child_id: string; parent_id: string }) =>
            expectOk(
                client.DELETE('/api/v1/parent-links/{child}/{parent}', {
                    params: { path: { child: vars.child_id, parent: vars.parent_id } },
                }),
            ),
        invalidate: () => [['tree']],
    })
}
