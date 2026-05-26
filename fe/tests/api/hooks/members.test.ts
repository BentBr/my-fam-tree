import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), PATCH: vi.fn(), DELETE: vi.fn() },
}))

import { client } from '@/api/client'
import { useMembers, useRevokeMember, useSetRole } from '@/api/hooks/members'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
    PATCH: ReturnType<typeof vi.fn>
    DELETE: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

beforeEach(() => {
    mocked.GET.mockReset()
    mocked.PATCH.mockReset()
    mocked.DELETE.mockReset()
    localStorage.clear()
})

describe('useMembers', () => {
    it('GETs /families/{family_id}/members and unwraps data.data.data', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.GET.mockResolvedValueOnce({
            data: {
                data: {
                    data: [
                        {
                            user_id: 'u-1',
                            email: 'a@b',
                            display_name: 'A B',
                            role: 'owner',
                            joined_at: '2026-01-01T00:00:00Z',
                        },
                    ],
                },
            },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useMembers())
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/families/{family_id}/members', {
            params: { path: { family_id: 'fam-1' } },
        })
        expect(result.data.value).toHaveLength(1)
        expect(result.data.value?.[0]?.role).toBe('owner')
    })

    it('is disabled when there is no active family', async () => {
        makeHookWrapper(() => useMembers())
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).not.toHaveBeenCalled()
    })
})

describe('useSetRole', () => {
    it('PATCHes /members/{user_id} and invalidates the members cache', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.PATCH.mockResolvedValueOnce({
            data: { data: { user_id: 'u-2', role: 'admin' } },
            error: undefined,
        })
        const { result, queryClient } = makeHookWrapper(() => useSetRole())
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        await result.mutateAsync({ userId: 'u-2', role: 'admin' })
        expect(mocked.PATCH).toHaveBeenCalledWith('/api/v1/families/{family_id}/members/{user_id}', {
            params: { path: { family_id: 'fam-1', user_id: 'u-2' } },
            body: { role: 'admin' },
        })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['members'] })
    })

    it('rejects when no active family is set', async () => {
        const { result } = makeHookWrapper(() => useSetRole())
        await expect(
            result.mutateAsync({ userId: 'u-2', role: 'admin' }),
        ).rejects.toThrow(/no active family/)
    })
})

describe('useRevokeMember', () => {
    it('DELETEs /members/{user_id} and invalidates the members cache', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result, queryClient } = makeHookWrapper(() => useRevokeMember())
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        await result.mutateAsync('u-2')
        expect(mocked.DELETE).toHaveBeenCalledWith('/api/v1/families/{family_id}/members/{user_id}', {
            params: { path: { family_id: 'fam-1', user_id: 'u-2' } },
        })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['members'] })
    })

    it('rejects on error', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const { result } = makeHookWrapper(() => useRevokeMember())
        await expect(result.mutateAsync('u-2')).rejects.toBeDefined()
    })
})
