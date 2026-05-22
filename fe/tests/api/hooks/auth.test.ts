import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({ client: { POST: vi.fn(), GET: vi.fn() } }))

import { client } from '@/api/client'
import { useConsumeMagicLink, useRequestMagicLink } from '@/api/hooks/auth'
import { useAuthStore } from '@/stores/auth'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    POST: ReturnType<typeof vi.fn>
    GET: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

describe('useRequestMagicLink', () => {
    beforeEach(() => {
        mocked.POST.mockReset()
    })

    it('POSTs to /auth/magic-link with the supplied email', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result } = makeHookWrapper(() => useRequestMagicLink())
        await result.mutateAsync('a@b')
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/auth/magic-link', { body: { email: 'a@b' } })
    })

    it('rejects when the response carries an error', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const { result } = makeHookWrapper(() => useRequestMagicLink())
        await expect(result.mutateAsync('a@b')).rejects.toBeDefined()
    })
})

describe('useConsumeMagicLink', () => {
    beforeEach(() => {
        mocked.POST.mockReset()
    })

    it('applies returned claims into auth store on success', async () => {
        mocked.POST.mockResolvedValueOnce({
            data: { data: { user_id: 'u-1', email: 'a@b', locale: 'en', families: [] } },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useConsumeMagicLink())
        await result.mutateAsync('tok-1')
        const auth = useAuthStore()
        expect(auth.status).toBe('authenticated')
        expect(auth.user?.email).toBe('a@b')
    })

    it('rejects on error and leaves auth untouched', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { code: 'invalid_token' } })
        const { result } = makeHookWrapper(() => useConsumeMagicLink())
        await expect(result.mutateAsync('bad')).rejects.toBeDefined()
        const auth = useAuthStore()
        expect(auth.status).toBe('anonymous')
    })
})
