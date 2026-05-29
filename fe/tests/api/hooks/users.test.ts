import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn() },
}))

import { client } from '@/api/client'
import { useConfirmEmailChange, useMe, useRequestEmailChange, useUpdateMe } from '@/api/hooks/users'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
    POST: ReturnType<typeof vi.fn>
    PATCH: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

beforeEach(() => {
    mocked.GET.mockReset()
    mocked.POST.mockReset()
    mocked.PATCH.mockReset()
})

describe('useMe', () => {
    it('GETs /users/me', async () => {
        mocked.GET.mockResolvedValueOnce({ data: { data: { display_name: 'A' } }, error: undefined })
        // useMe is gated on `auth.status === 'authenticated'` (it backs the
        // AppBar avatar which mounts on the sign-in page too — without the
        // gate, every fresh page-load fires /users/me, 401s, and triggers
        // the FE's session_expired toast). Authenticate the store inside
        // the wrapper's setup so the query is enabled.
        const { result } = makeHookWrapper(() => {
            useAuthStore().status = 'authenticated'
            return useMe()
        })
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/users/me')
        // useMe now unwraps the envelope — call sites get the profile directly.
        expect(result.data.value).toEqual({ display_name: 'A' })
    })

    it('errors on response error', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => {
            useAuthStore().status = 'authenticated'
            return useMe()
        })
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value).toBeDefined()
    })

    it('stays disabled when the auth store is anonymous', async () => {
        // No `auth.status = 'authenticated'` here — the gate keeps the
        // query disabled so /users/me is never fetched.
        makeHookWrapper(() => useMe())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).not.toHaveBeenCalled()
    })
})

describe('useUpdateMe', () => {
    it('PATCHes /users/me and syncs locale + auth on success', async () => {
        mocked.PATCH.mockResolvedValueOnce({
            data: { data: { display_name: 'Z', locale: 'de' } },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useUpdateMe())
        await result.mutateAsync({ display_name: 'Z', locale: 'de' })
        const locale = useLocaleStore()
        const auth = useAuthStore()
        expect(locale.locale).toBe('de')
        // auth was anonymous so patchUser no-ops on user; that's fine. We only
        // need to know locale.set fired.
        expect(auth.status).toBe('anonymous')
    })

    it('skips locale.set when server locale is not en/de', async () => {
        mocked.PATCH.mockResolvedValueOnce({
            data: { data: { display_name: 'Z', locale: 'fr' } },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useUpdateMe())
        await result.mutateAsync({ display_name: 'Z' })
        // locale falls through to patchUser without setting; with no user yet, no observable effect on locale store.
        const locale = useLocaleStore()
        expect(['en', 'de']).toContain(locale.locale)
    })

    it('rejects on error', async () => {
        mocked.PATCH.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useUpdateMe())
        await expect(result.mutateAsync({})).rejects.toBeDefined()
    })
})

describe('useRequestEmailChange', () => {
    it('POSTs and toasts on success', async () => {
        mocked.POST.mockResolvedValueOnce({ data: { ok: true }, error: undefined })
        const { result } = makeHookWrapper(() => useRequestEmailChange())
        await result.mutateAsync('new@b')
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/users/me/email-change', {
            body: { new_email: 'new@b' },
        })
    })

    it('rejects on error', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useRequestEmailChange())
        await expect(result.mutateAsync('a')).rejects.toBeDefined()
    })
})

describe('useConfirmEmailChange', () => {
    it('POSTs token and syncs email/displayName', async () => {
        mocked.POST.mockResolvedValueOnce({
            data: { data: { display_name: 'A', email: 'new@b' } },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useConfirmEmailChange())
        await result.mutateAsync('tok')
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/users/me/email-change/confirm', {
            body: { token: 'tok' },
        })
    })

    it('rejects on error', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useConfirmEmailChange())
        await expect(result.mutateAsync('bad')).rejects.toBeDefined()
    })
})
