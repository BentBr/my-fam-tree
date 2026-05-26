import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn(), DELETE: vi.fn() },
}))

import { client } from '@/api/client'
import {
    useBeginOwnerTransfer,
    useCancelOwnerTransfer,
    useConfirmOwnerTransfer,
    useOwnerTransfer,
} from '@/api/hooks/owner_transfer'

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

describe('useOwnerTransfer', () => {
    it('GETs /transfer-owner and returns the row when one is pending', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.GET.mockResolvedValueOnce({
            data: {
                data: {
                    id: 't-1',
                    from_user_id: 'u-1',
                    to_user_id: 'u-2',
                    from_confirmed: true,
                    to_confirmed: false,
                    expires_at: '2026-12-01T00:00:00Z',
                },
            },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useOwnerTransfer())
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/families/{family_id}/transfer-owner', {
            params: { path: { family_id: 'fam-1' } },
        })
        expect(result.data.value?.id).toBe('t-1')
        expect(result.data.value?.to_confirmed).toBe(false)
    })

    it('resolves to null when no transfer is pending', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.GET.mockResolvedValueOnce({ data: { data: null }, error: undefined })
        const { result } = makeHookWrapper(() => useOwnerTransfer())
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(result.data.value).toBeNull()
    })

    it('is disabled when there is no active family', async () => {
        makeHookWrapper(() => useOwnerTransfer())
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).not.toHaveBeenCalled()
    })
})

describe('useBeginOwnerTransfer', () => {
    it('POSTs to /transfer-owner with to_user_id and invalidates caches', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.POST.mockResolvedValueOnce({
            data: {
                data: {
                    id: 't-2',
                    from_user_id: 'u-1',
                    to_user_id: 'u-2',
                    from_confirmed: false,
                    to_confirmed: false,
                    expires_at: '2026-12-01T00:00:00Z',
                },
            },
            error: undefined,
        })
        const { result, queryClient } = makeHookWrapper(() => useBeginOwnerTransfer())
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        const out = await result.mutateAsync('u-2')
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/families/{family_id}/transfer-owner', {
            params: { path: { family_id: 'fam-1' } },
            body: { to_user_id: 'u-2' },
        })
        expect(out.id).toBe('t-2')
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['owner-transfer'] })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['members'] })
    })

    it('rejects when no active family is set', async () => {
        const { result } = makeHookWrapper(() => useBeginOwnerTransfer())
        await expect(result.mutateAsync('u-2')).rejects.toThrow(/no active family/)
    })
})

describe('useConfirmOwnerTransfer', () => {
    it('POSTs to /transfer-owner/confirm with the token and invalidates caches', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.POST.mockResolvedValueOnce({
            data: {
                data: {
                    id: 't-2',
                    from_user_id: 'u-1',
                    to_user_id: 'u-2',
                    from_confirmed: true,
                    to_confirmed: true,
                    expires_at: '2026-12-01T00:00:00Z',
                },
            },
            error: undefined,
        })
        const { result, queryClient } = makeHookWrapper(() => useConfirmOwnerTransfer())
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        const out = await result.mutateAsync('tok-abc')
        expect(mocked.POST).toHaveBeenCalledWith(
            '/api/v1/families/{family_id}/transfer-owner/confirm',
            {
                params: { path: { family_id: 'fam-1' } },
                body: { token: 'tok-abc' },
            },
        )
        expect(out.from_confirmed).toBe(true)
        expect(out.to_confirmed).toBe(true)
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['owner-transfer'] })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['members'] })
    })

    it('rejects when no active family is set', async () => {
        const { result } = makeHookWrapper(() => useConfirmOwnerTransfer())
        await expect(result.mutateAsync('tok-abc')).rejects.toThrow(/no active family/)
    })
})

describe('useCancelOwnerTransfer', () => {
    it('DELETEs /transfer-owner and invalidates the owner-transfer cache', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result, queryClient } = makeHookWrapper(() => useCancelOwnerTransfer())
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        await result.mutateAsync()
        expect(mocked.DELETE).toHaveBeenCalledWith('/api/v1/families/{family_id}/transfer-owner', {
            params: { path: { family_id: 'fam-1' } },
        })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['owner-transfer'] })
    })

    it('rejects on error', async () => {
        localStorage.setItem('my-family:activeFamily', 'fam-1')
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const { result } = makeHookWrapper(() => useCancelOwnerTransfer())
        await expect(result.mutateAsync()).rejects.toBeDefined()
    })
})
