import { useMutation, useQueryClient } from '@tanstack/vue-query'

import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'

/**
 * Single unwrap for every openapi-fetch call.
 *
 * `await`-ing the call may already reject with an `ApiClientError` — the
 * `errorTranslator` middleware throws that for any `application/problem+json`
 * response, so error responses never reach the body of this function. What
 * DOES reach here is either a success (`{ data }`) or a non-problem error
 * surfaced by openapi-fetch as `{ error }`; we re-throw the latter and
 * guard against an empty success body. Hooks call `unwrap(client.GET(...))`
 * instead of repeating the throw/empty checks themselves.
 */
export async function unwrap<T>(
    call: Promise<{ data?: { data: T } | undefined; error?: unknown }>,
): Promise<T> {
    const { data, error } = await call
    if (error !== undefined) throw error
    if (data === undefined) throw new Error('empty response from API')
    return data.data
}

type SuccessMessage<TVars, TData> = string | ((vars: TVars, data: TData) => string)

/**
 * Thin `useMutation` wrapper that centralises the two things every
 * mutation hook used to hand-roll:
 *   - the success toast (an i18n key, or a fn of the vars/result so
 *     direction-dependent messages like favourite marked/unmarked work),
 *   - query invalidation on success (a list of query-key prefixes).
 *
 * Errors are intentionally NOT handled here — they bubble to the
 * queryClient's `MutationCache.onError` (the single report point), so
 * error/validation messaging lives in exactly one place.
 */
export function useApiMutation<TVars, TData>(opts: {
    mutationFn: (vars: TVars) => Promise<TData>
    success?: SuccessMessage<TVars, TData>
    invalidate?: (vars: TVars, data: TData) => unknown[][]
    onSuccess?: (vars: TVars, data: TData) => void
}) {
    const qc = useQueryClient()
    const ui = useUiStore()
    return useMutation({
        mutationFn: opts.mutationFn,
        onSuccess: (data: TData, vars: TVars) => {
            if (opts.success !== undefined) {
                const key = typeof opts.success === 'function' ? opts.success(vars, data) : opts.success
                ui.pushToast({ kind: 'success', message: i18n.global.t(key) })
            }
            if (opts.invalidate !== undefined) {
                for (const queryKey of opts.invalidate(vars, data)) {
                    void qc.invalidateQueries({ queryKey })
                }
            }
            opts.onSuccess?.(vars, data)
        },
    })
}
