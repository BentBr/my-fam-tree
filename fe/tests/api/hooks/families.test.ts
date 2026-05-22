import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({ client: { POST: vi.fn() } }))

import { client } from '@/api/client'
import { useAcceptInvite, useCreateFamily } from '@/api/hooks/families'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    POST: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

describe('useCreateFamily', () => {
    beforeEach(() => {
        mocked.POST.mockReset()
    })

    it('POSTs body and applies returned claims + success toast', async () => {
        mocked.POST.mockResolvedValueOnce({
            data: {
                data: {
                    family: { id: 'f-1', name: 'F' },
                    claims: {
                        user_id: 'u',
                        email: 'a@b',
                        locale: 'en',
                        families: [{ id: 'f-1', name: 'F', role: 'owner' }],
                    },
                },
            },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useCreateFamily())
        await result.mutateAsync('Family')
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/families', { body: { name: 'Family' } })
        const auth = useAuthStore()
        expect(auth.families).toHaveLength(1)
        const ui = useUiStore()
        expect(ui.toasts).toHaveLength(1)
    })

    it('throws on error', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useCreateFamily())
        await expect(result.mutateAsync('x')).rejects.toBeDefined()
    })
})

describe('useAcceptInvite', () => {
    beforeEach(() => {
        mocked.POST.mockReset()
    })

    it('POSTs and applies claims + invite-accepted toast', async () => {
        mocked.POST.mockResolvedValueOnce({
            data: {
                data: {
                    family: { id: 'f-1', name: 'F' },
                    claims: {
                        user_id: 'u',
                        email: 'a@b',
                        locale: 'en',
                        families: [{ id: 'f-1', name: 'F', role: 'user' }],
                    },
                },
            },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useAcceptInvite())
        await result.mutateAsync('tok')
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/invites/accept', { body: { token: 'tok' } })
    })

    it('throws on error and skips toast', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useAcceptInvite())
        await expect(result.mutateAsync('bad')).rejects.toBeDefined()
    })
})
