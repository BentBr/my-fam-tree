import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn(), DELETE: vi.fn() },
}))

import { client } from '@/api/client'
import { useCancelInvite, useCreateInvite, useInvites } from '@/api/hooks/invites'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
    POST: ReturnType<typeof vi.fn>
    DELETE: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

beforeEach(() => {
    mocked.GET.mockReset()
    mocked.POST.mockReset()
    mocked.DELETE.mockReset()
    localStorage.clear()
})

describe('useInvites', () => {
    it('GETs /families/{id}/invites and unwraps the nested data envelope', async () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'fam-1')
        mocked.GET.mockResolvedValueOnce({
            data: {
                data: {
                    data: [
                        {
                            id: 'i-1',
                            email: 'a@b',
                            role: 'user',
                            person_id: null,
                            expires_at: '2026-12-01T00:00:00Z',
                            invited_by: 'u-1',
                        },
                    ],
                },
            },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useInvites())
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/families/{id}/invites', {
            params: { path: { id: 'fam-1' } },
        })
        expect(result.data.value).toHaveLength(1)
        expect(result.data.value?.[0]?.email).toBe('a@b')
    })

    it('is disabled when there is no active family', async () => {
        makeHookWrapper(() => useInvites())
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).not.toHaveBeenCalled()
    })
})

describe('useCreateInvite', () => {
    it('POSTs body with the active family id and invalidates the invites cache', async () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'fam-1')
        mocked.POST.mockResolvedValueOnce({
            data: { data: { id: 'i-1', email: 'a@b', role: 'user' } },
            error: undefined,
        })
        const { result, queryClient } = makeHookWrapper(() => useCreateInvite())
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        await result.mutateAsync({ email: 'a@b', role: 'user', personId: 'p-1' })
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/families/{id}/invites', {
            params: { path: { id: 'fam-1' } },
            body: { email: 'a@b', role: 'user', person_id: 'p-1' },
        })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['invites'] })
    })

    it('coerces a missing personId to null in the body', async () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'fam-1')
        mocked.POST.mockResolvedValueOnce({
            data: { data: { id: 'i-2' } },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useCreateInvite())
        await result.mutateAsync({ email: 'b@c', role: 'admin' })
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/families/{id}/invites', {
            params: { path: { id: 'fam-1' } },
            body: { email: 'b@c', role: 'admin', person_id: null },
        })
    })

    it('rejects when no active family is set', async () => {
        const { result } = makeHookWrapper(() => useCreateInvite())
        await expect(result.mutateAsync({ email: 'a@b', role: 'user' })).rejects.toThrow(/no active family/)
    })
})

describe('useCancelInvite', () => {
    it('DELETEs /invites/{invite_id} and invalidates the invites cache', async () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'fam-1')
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result, queryClient } = makeHookWrapper(() => useCancelInvite())
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        await result.mutateAsync('i-1')
        expect(mocked.DELETE).toHaveBeenCalledWith('/api/v1/families/{id}/invites/{invite_id}', {
            params: { path: { id: 'fam-1', invite_id: 'i-1' } },
        })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['invites'] })
    })

    it('rejects on error', async () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'fam-1')
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const { result } = makeHookWrapper(() => useCancelInvite())
        await expect(result.mutateAsync('i-1')).rejects.toBeDefined()
    })
})
