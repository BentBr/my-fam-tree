import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn() },
}))

import { client } from '@/api/client'
import { useUpcoming, type UpcomingFilter } from '@/api/hooks/upcoming'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

beforeEach(() => {
    mocked.GET.mockReset()
})

describe('useUpcoming', () => {
    it('GETs /upcoming without a filter param when filter=all', async () => {
        mocked.GET.mockResolvedValueOnce({
            data: { data: [{ kind: 'birthday', next_date: '2026-12-01', years: 30, label: 'A' }] },
            error: undefined,
        })
        const filter = ref<UpcomingFilter>('all')
        const { result } = makeHookWrapper(() => useUpcoming(filter))
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/upcoming', {
            params: { query: undefined },
        })
        expect(result.data.value?.[0]?.kind).toBe('birthday')
    })

    it('passes ?filter=birthday when filter=birthday', async () => {
        mocked.GET.mockResolvedValueOnce({ data: { data: [] }, error: undefined })
        const filter = ref<UpcomingFilter>('birthday')
        makeHookWrapper(() => useUpcoming(filter))
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/upcoming', {
            params: { query: { filter: 'birthday' } },
        })
    })

    it('passes ?filter=anniversary when filter=anniversary', async () => {
        mocked.GET.mockResolvedValueOnce({ data: { data: [] }, error: undefined })
        const filter = ref<UpcomingFilter>('anniversary')
        makeHookWrapper(() => useUpcoming(filter))
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/upcoming', {
            params: { query: { filter: 'anniversary' } },
        })
    })

    it('errors on empty data', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: undefined })
        const filter = ref<UpcomingFilter>('all')
        const { result } = makeHookWrapper(() => useUpcoming(filter))
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value?.message).toMatch(/empty response/)
    })

    it('errors on response error', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const filter = ref<UpcomingFilter>('all')
        const { result } = makeHookWrapper(() => useUpcoming(filter))
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value).toBeDefined()
    })
})
