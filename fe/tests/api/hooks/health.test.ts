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

    it('runs a query with key ["health"] and returns body + network round-trip', async () => {
        mocked.GET.mockResolvedValueOnce({ data: { data: { version: '1.0' } }, error: undefined })
        const { result } = makeHookWrapper(() => useHealth())
        // Let queryFn settle.
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/health')
        // useHealth now appends a FE-measured `network_round_trip_ms`
        // alongside the BE envelope. The exact ms is wall-clock-
        // dependent so we only assert SHAPE: `data` propagated +
        // `network_round_trip_ms` present + non-negative.
        const value = result.data.value as { data?: { version?: string }; network_round_trip_ms?: number }
        expect(value.data).toEqual({ version: '1.0' })
        expect(typeof value.network_round_trip_ms).toBe('number')
        expect(value.network_round_trip_ms ?? -1).toBeGreaterThanOrEqual(0)
    })

    it('throws when the response is an error', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'fail' } })
        const { result } = makeHookWrapper(() => useHealth())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value).toBeDefined()
    })
})
