import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'

import { ApiClientError, type ApiErrorBody } from '@/api/errors'
import { queryClient } from '@/api/queryClient'
import { useUiStore } from '@/stores/ui'

function body(over: Partial<ApiErrorBody> = {}): ApiErrorBody {
    return {
        type: 'about:blank',
        title: 'Bad',
        status: 400,
        code: 'validation_failed',
        ...over,
    } as ApiErrorBody
}

describe('queryClient onError handler (via QueryCache)', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    function fireError(err: unknown): void {
        // Directly invoke the QueryCache's onError; the handler is what we
        // care about. queryClient.getQueryCache() exposes the cache instance
        // and TanStack Query's notify path calls this onError option.
        const cache = queryClient.getQueryCache()
        const cacheOpts = cache.config as { onError?: (e: unknown) => void }
        cacheOpts.onError?.(err)
    }

    it('pushes an error toast for ApiClientError', () => {
        const ui = useUiStore()
        fireError(new ApiClientError(body({ code: 'validation_failed' })))
        expect(ui.toasts).toHaveLength(1)
        expect(ui.toasts[0]?.kind).toBe('error')
        expect(ui.toasts[0]?.code).toBe('validation_failed')
    })

    it('drops 401 errors silently (auth refresh middleware owns them)', () => {
        const ui = useUiStore()
        fireError(new ApiClientError(body({ status: 401, code: 'auth_token_expired' })))
        expect(ui.toasts).toHaveLength(0)
    })

    it('forwards instance as requestId when present', () => {
        const ui = useUiStore()
        fireError(new ApiClientError(body({ instance: 'req-xyz' })))
        expect(ui.toasts[0]?.requestId).toBe('req-xyz')
    })

    it('falls back to Error.message for non-Api errors', () => {
        const ui = useUiStore()
        fireError(new Error('boom'))
        expect(ui.toasts[0]?.message).toBe('boom')
    })

    it('coerces unknown thrown values via String()', () => {
        const ui = useUiStore()
        fireError({ weird: true })
        expect(ui.toasts[0]?.message).toContain('object')
    })
})

describe('queryClient defaults', () => {
    it('disables retry on queries and mutations', () => {
        const opts = queryClient.getDefaultOptions()
        expect(opts.queries?.retry).toBe(0)
        expect(opts.mutations?.retry).toBe(0)
    })
})
