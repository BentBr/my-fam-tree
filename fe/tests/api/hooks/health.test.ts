import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { client } from '@/api/client'
import { useHealth } from '@/api/hooks/health'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

describe('useHealth', () => {
    beforeEach(() => {
        mocked.GET.mockReset()
    })

    it('runs a query with key ["health"] and unwraps data', async () => {
        mocked.GET.mockResolvedValueOnce({ data: { data: { version: '1.0' } }, error: undefined })
        const { result } = makeHookWrapper(() => useHealth())
        // Let queryFn settle.
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/health')
        expect(result.data.value).toEqual({ data: { version: '1.0' } })
    })

    it('throws when the response is an error', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'fail' } })
        const { result } = makeHookWrapper(() => useHealth())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value).toBeDefined()
    })
})
