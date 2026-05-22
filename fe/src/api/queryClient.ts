import { MutationCache, QueryCache, QueryClient } from '@tanstack/vue-query'

import { i18n } from '@/i18n'
import { useUiStore, type Toast } from '@/stores/ui'

import { ApiClientError, type FieldViolation } from './errors'

// Module-evaluation time runs BEFORE Pinia is installed on the app (queryClient
// is imported from main.ts prior to `app.use(createPinia())`). To stay safe we
// only resolve the Pinia store inside the handler bodies, never at module top.

// Translate one of three layers, in priority order:
//   1. RFC 7807 `fields[]` — per-field validation codes (semantic, like
//      `validation.parent_not_older_than_child`). Most actionable for users;
//      shown as one message per field separated by '; '.
//   2. The top-level `code` (an `ErrorCode` enum from the backend). Maps to
//      `errorCodes.<code>` in the catalog; covers conflicts (409),
//      authorization (403), rate-limit, etc.
//   3. The server-provided `body.title` as a last-resort English fallback so
//      we never display a blank toast.
function translateFieldViolation(v: FieldViolation): string {
    const params = (v.params ?? {}) as Record<string, unknown>
    if (i18n.global.te(v.code)) {
        return i18n.global.t(v.code, params)
    }
    return v.message
}

function translateError(err: ApiClientError): string {
    const fields = err.body.fields ?? []
    if (fields.length > 0) {
        return fields.map(translateFieldViolation).join('; ')
    }
    const codeKey = `errorCodes.${err.code}`
    if (i18n.global.te(codeKey)) {
        return i18n.global.t(codeKey)
    }
    return err.body.title
}

function reportError(error: unknown): void {
    // 401s are absorbed by the auth refresh middleware / route guard. Surfacing
    // a toast for them would race with the silent token refresh.
    if (error instanceof ApiClientError) {
        if (error.status === 401) return
        const t: Omit<Toast, 'id'> = { kind: 'error', message: translateError(error) }
        t.code = error.body.code
        // RFC 7807 `instance` is the request URL/id — useful for support
        // correlation. The schema types it as `string | null | undefined`.
        if (error.body.instance !== undefined && error.body.instance !== null) {
            t.requestId = error.body.instance
        }
        useUiStore().pushToast(t)
        return
    }
    if (error instanceof Error) {
        useUiStore().pushToast({ kind: 'error', message: error.message })
        return
    }
    useUiStore().pushToast({ kind: 'error', message: String(error) })
}

export const queryClient = new QueryClient({
    queryCache: new QueryCache({
        onError: reportError,
    }),
    mutationCache: new MutationCache({
        onError: reportError,
    }),
    defaultOptions: {
        queries: {
            retry: 0,
            // Refetch-on-focus produces noisy toasts during dev. Opt in per
            // query when needed.
            refetchOnWindowFocus: false,
        },
        mutations: {
            retry: 0,
        },
    },
})
