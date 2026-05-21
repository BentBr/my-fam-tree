import { MutationCache, QueryCache, QueryClient } from '@tanstack/vue-query'

import { useUiStore, type Toast } from '@/stores/ui'

import { ApiClientError } from './errors'

// Module-evaluation time runs BEFORE Pinia is installed on the app (queryClient
// is imported from main.ts prior to `app.use(createPinia())`). To stay safe we
// only resolve the Pinia store inside the handler bodies, never at module top.
function reportError(error: unknown): void {
    // 401s are absorbed by the auth refresh middleware / route guard. Surfacing
    // a toast for them would race with the silent token refresh.
    if (error instanceof ApiClientError) {
        if (error.status === 401) return
        const t: Omit<Toast, 'id'> = { kind: 'error', message: error.body.title }
        if (error.body.code !== undefined) t.code = error.body.code
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
