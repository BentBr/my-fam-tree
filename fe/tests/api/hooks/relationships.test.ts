import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn() },
}))

import { client } from '@/api/client'
import { useAddParentLink, useCreatePartnership, useTree } from '@/api/hooks/relationships'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
    POST: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

beforeEach(() => {
    mocked.GET.mockReset()
    mocked.POST.mockReset()
})

describe('useTree', () => {
    it('GETs /relationships and unwraps data', async () => {
        mocked.GET.mockResolvedValueOnce({
            data: { data: { nodes: [], parent_edges: [], partner_edges: [] } },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useTree())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/relationships')
        expect(result.data.value).toEqual({ nodes: [], parent_edges: [], partner_edges: [] })
    })

    it('errors when data is empty', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result } = makeHookWrapper(() => useTree())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value?.message).toMatch(/empty response/)
    })

    it('errors on response error', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useTree())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value).toBeDefined()
    })
})

describe('useAddParentLink', () => {
    it('POSTs to /parent-links', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result } = makeHookWrapper(() => useAddParentLink())
        await result.mutateAsync({ child_id: 'c', parent_id: 'p', kind: 'biological' })
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/parent-links', {
            body: { child_id: 'c', parent_id: 'p', kind: 'biological' },
        })
    })

    it('rejects on error', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useAddParentLink())
        await expect(result.mutateAsync({ child_id: 'c', parent_id: 'p', kind: 'b' })).rejects.toBeDefined()
    })
})

describe('useCreatePartnership', () => {
    it('POSTs to /partnerships', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result } = makeHookWrapper(() => useCreatePartnership())
        await result.mutateAsync({ partner_a_id: 'a', partner_b_id: 'b', kind: 'partnership' })
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/partnerships', {
            body: { partner_a_id: 'a', partner_b_id: 'b', kind: 'partnership' },
        })
    })

    it('rejects on error', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useCreatePartnership())
        await expect(result.mutateAsync({ partner_a_id: 'a', partner_b_id: 'b', kind: 'x' })).rejects.toBeDefined()
    })
})
