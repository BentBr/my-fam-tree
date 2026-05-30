import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, type Ref } from 'vue'

import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'

import { client } from '../client'
import { expectOk, unwrap, useApiMutation } from '../request'

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
    email?: string
    phone?: string
    street?: string
    house_number?: string
    zip?: string
    city?: string
    country?: string
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
    email?: string | null
    phone?: string | null
    street?: string | null
    house_number?: string | null
    zip?: string | null
    city?: string | null
    country?: string | null
    linked_user_id?: string | null
}

// `PersonsListResponseBody` on the wire is `{ data: PersonView[], meta? }` — the
// shared `unwrap()` returns `data.data` (the `PersonView[]` array) directly so
// call sites don't have to peel an extra wrapping layer that exists purely for
// response metadata. `unwrap` also handles the `error` re-throw and the
// empty-body guard (a protocol-level surprise like an empty 204).
export function useListPersons() {
    return useQuery({
        queryKey: ['persons'],
        queryFn: () => unwrap(client.GET('/api/v1/persons')),
    })
}

/**
 * Async fetch of a single `PersonView` by id. Backs the tree drawer so the
 * "click → drawer" path always renders the latest server state (vs the
 * possibly-stale `useListPersons` cache shared with the page).
 *
 * The query key is reactive: when the caller flips the ref to a new id the
 * query swaps cleanly; setting it to `null` disables the query, which is
 * what the drawer wants when nothing is selected.
 */
export function useGetPerson(id: Ref<string | null>) {
    return useQuery({
        queryKey: ['person', id] as const,
        // `enabled` short-circuits the fetch while id is null — react-query
        // still returns a usable result object, just `data === undefined`.
        enabled: computed(() => id.value !== null && id.value !== ''),
        queryFn: () => {
            const pid = id.value
            // `enabled` guards against this, but the queryFn type still needs
            // to refuse a null id at runtime so the URL builder doesn't see
            // the literal string `null`.
            if (pid === null || pid === '') throw new Error('useGetPerson: id is null')
            return unwrap(client.GET('/api/v1/persons/{id}', { params: { path: { id: pid } } }))
        },
    })
}

export function useCreatePerson() {
    return useApiMutation({
        mutationFn: (input: PersonInput) => unwrap(client.POST('/api/v1/persons', { body: input })),
        success: 'toasts.person_created',
        invalidate: () => [['persons'], ['tree']],
    })
}

export function useUpdatePerson() {
    return useApiMutation({
        mutationFn: (vars: { id: string; input: PersonUpdateInput }) =>
            unwrap(client.PATCH('/api/v1/persons/{id}', { params: { path: { id: vars.id } }, body: vars.input })),
        success: 'toasts.person_updated',
        // Also refresh the per-person GET so the drawer sees the new fields
        // without a manual refetch. The query key includes a ref-wrapping id,
        // so the broad `['person', id]` prefix matches it structurally.
        invalidate: (vars) => [['persons'], ['tree'], ['person', vars.id]],
    })
}

export function useDeletePerson() {
    return useApiMutation({
        mutationFn: (id: string) => expectOk(client.DELETE('/api/v1/persons/{id}', { params: { path: { id } } })),
        success: 'toasts.person_deleted',
        invalidate: () => [['persons'], ['tree']],
    })
}

/**
 * Self-claim a person row. The BE always links the calling user, never
 * anyone else — see the route doc on `crates/api/src/routes/persons.rs`
 * for the consent rationale (PATCH /persons/{id} would allow linking any
 * user, which is fine for admin self-management but leaks consent for
 * other users; this endpoint is the consent-safe shortcut).
 *
 * Invalidates the same caches as `useUpdatePerson` so the linked-account
 * chip, the tree's "this is you" highlight, and the per-person GET all
 * refresh in one tick.
 */
export function useClaimPerson() {
    return useApiMutation({
        mutationFn: (id: string) => unwrap(client.POST('/api/v1/persons/{id}/claim', { params: { path: { id } } })),
        success: 'toasts.person_claimed',
        invalidate: (id) => [['persons'], ['tree'], ['person', id]],
    })
}

/**
 * Upload a new photo for a person. `file` is the raw File from an
 * `<input type="file">`; we wrap it in a FormData under the single field
 * name `file` (the BE accepts exactly that, see `routes/person_photos.rs`).
 *
 * The body is passed to openapi-fetch as a FormData instance — fetch sets
 * the multipart boundary automatically, so we MUST NOT also send an
 * explicit Content-Type header (would clobber the boundary parameter).
 * The `bodySerializer` short-circuit returns the FormData unchanged.
 *
 * Invalidates the per-person GET + the persons list so every render of
 * the person sees the new `photo_url` immediately.
 */
export function useSetPersonPhoto() {
    return useApiMutation({
        mutationFn: (vars: { id: string; file: File }) => {
            const fd = new FormData()
            fd.append('file', vars.file)
            return unwrap(
                client.POST('/api/v1/persons/{id}/photo', {
                    params: { path: { id: vars.id } },
                    // openapi-fetch's body for multipart accepts FormData;
                    // we cast to bypass the schema-generated `string` body
                    // type (utoipa surfaces multipart bodies as `String`).
                    body: fd as unknown as string,
                    bodySerializer: (b: unknown) => b as BodyInit,
                }),
            )
        },
        success: 'toasts.person_photo_set',
        invalidate: (vars) => [['persons'], ['tree'], ['person', vars.id]],
    })
}

export function useClearPersonPhoto() {
    return useApiMutation({
        mutationFn: (id: string) => expectOk(client.DELETE('/api/v1/persons/{id}/photo', { params: { path: { id } } })),
        success: 'toasts.person_photo_cleared',
        invalidate: (id) => [['persons'], ['tree'], ['person', id]],
    })
}

/**
 * Shape of a tree node as it lives in the `['tree']` query cache. Mirrors
 * the BE's `TreeNode` schema; declared inline (instead of imported from
 * `schema.d.ts`) so the optimistic-update path stays self-contained and
 * doesn't drag the full generated type tree into this file.
 */
interface CachedTreeNode {
    id: string
    is_favourite_for_me: boolean
    [k: string]: unknown
}
interface CachedTreePayload {
    nodes: CachedTreeNode[]
    [k: string]: unknown
}

/**
 * Per-user favourite toggle. The mark is private — two members of the
 * same family see independent state on the same person row — so this
 * mutation never invalidates the family-wide caches the way person CRUD
 * does. We optimistically flip `is_favourite_for_me` on the cached
 * `['tree']` payload + the cached `['person', id]` GET so the UI feels
 * instant; on rejection both writes roll back via the snapshot taken
 * inside `onMutate`.
 *
 * Toasts: a brief `favourite_marked` / `favourite_unmarked` so the
 * action is acknowledged. The error toast pipeline in `queryClient.ts`
 * still surfaces server failures.
 */
export function useSetFavourite() {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        // Kept on raw `useMutation` (not `useApiMutation`) because it owns
        // an optimistic onMutate + rollback onError that the wrapper doesn't
        // expose. The network call still routes through `unwrap()` so the
        // error/empty handling stays consistent with every other hook.
        mutationFn: (vars: { id: string; isFavourite: boolean }) =>
            unwrap(
                client.PATCH('/api/v1/persons/{id}/favourite', {
                    params: { path: { id: vars.id } },
                    body: { is_favourite: vars.isFavourite },
                }),
            ),
        onMutate: async (vars) => {
            await qc.cancelQueries({ queryKey: ['tree'] })
            await qc.cancelQueries({ queryKey: ['person', vars.id] })
            const prevTree = qc.getQueryData<CachedTreePayload>(['tree'])
            // The per-person GET uses a reactive id ref as part of its key —
            // tanstack-query keys are compared structurally, so we use the
            // broad prefix here and roll back the same way.
            const prevPerson = qc.getQueriesData<{ is_favourite_for_me?: boolean }>({
                queryKey: ['person', vars.id],
            })
            if (prevTree !== undefined) {
                qc.setQueryData<CachedTreePayload>(['tree'], {
                    ...prevTree,
                    nodes: prevTree.nodes.map((n) =>
                        n.id === vars.id ? { ...n, is_favourite_for_me: vars.isFavourite } : n,
                    ),
                })
            }
            for (const [key, value] of prevPerson) {
                if (value !== undefined && value !== null) {
                    qc.setQueryData(key, { ...value, is_favourite_for_me: vars.isFavourite })
                }
            }
            return { prevTree, prevPerson }
        },
        onError: (_err, _vars, ctx) => {
            if (ctx === undefined) return
            if (ctx.prevTree !== undefined) {
                qc.setQueryData(['tree'], ctx.prevTree)
            }
            for (const [key, value] of ctx.prevPerson) {
                qc.setQueryData(key, value)
            }
        },
        onSuccess: (_data, vars) => {
            ui.pushToast({
                kind: 'success',
                message: i18n.global.t(vars.isFavourite ? 'toasts.favourite_marked' : 'toasts.favourite_unmarked'),
            })
        },
    })
}
