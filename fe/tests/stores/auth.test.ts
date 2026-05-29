import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { ApiClientError, type ApiErrorBody } from '@/api/errors'
import type { ClaimsPayload } from '@/api/types'

// Mock the openapi-fetch client + the router import inside @/api/client.
vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn() },
}))
vi.mock('@/router', () => ({ router: { replace: vi.fn() } }))

import { client } from '@/api/client'
import { useAuthStore } from '@/stores/auth'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
    POST: ReturnType<typeof vi.fn>
}

const mocked = client as unknown as MockedClient

function mockStorage(): void {
    const store: Record<string, string> = {}
    const api = {
        getItem: (k: string) => store[k] ?? null,
        setItem: (k: string, v: string) => {
            store[k] = v
        },
        removeItem: (k: string) => {
            delete store[k]
        },
        key: (i: number) => Object.keys(store)[i] ?? null,
        get length() {
            return Object.keys(store).length
        },
    }
    vi.stubGlobal('localStorage', api)
    vi.stubGlobal('sessionStorage', api)
}

function claims(over: Partial<ClaimsPayload> = {}): ClaimsPayload {
    return {
        user_id: 'u-1',
        email: 'a@b',
        locale: 'en',
        families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        ...over,
    } as ClaimsPayload
}

function errBody(over: Partial<ApiErrorBody> = {}): ApiErrorBody {
    return {
        type: 'about:blank',
        title: 'x',
        status: 401,
        code: 'auth_required',
        ...over,
    } as ApiErrorBody
}

describe('auth store', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        mockStorage()
        mocked.GET.mockReset()
        mocked.POST.mockReset()
    })

    it('starts anonymous with no user / families', () => {
        const auth = useAuthStore()
        expect(auth.status).toBe('anonymous')
        expect(auth.user).toBeNull()
        expect(auth.families).toEqual([])
    })

    it('applyClaimsPayload(null) clears state', () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload(claims())
        auth.applyClaimsPayload(null)
        expect(auth.status).toBe('anonymous')
        expect(auth.user).toBeNull()
    })

    it('applyClaimsPayload populates user/families and locale', () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload(claims({ locale: 'de' }))
        expect(auth.status).toBe('authenticated')
        expect(auth.user?.email).toBe('a@b')
        expect(auth.user?.locale).toBe('de')
        expect(auth.families).toHaveLength(1)
    })

    it('applyClaimsPayload normalises an unknown locale to "en"', () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload(claims({ locale: 'fr' as 'en' }))
        expect(auth.user?.locale).toBe('en')
    })

    it('patchUser merges into user state', () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload(claims())
        auth.patchUser({ displayName: 'Alice', email: 'new@b' })
        expect(auth.user?.displayName).toBe('Alice')
        expect(auth.user?.email).toBe('new@b')
    })

    it('patchUser is a no-op when anonymous', () => {
        const auth = useAuthStore()
        auth.patchUser({ displayName: 'Z' })
        expect(auth.user).toBeNull()
    })

    it('hydrate populates state on 200', async () => {
        mocked.GET.mockResolvedValueOnce({ data: { data: claims() }, error: undefined })
        const auth = useAuthStore()
        await auth.hydrate()
        expect(auth.status).toBe('authenticated')
    })

    it('hydrate clears state on 401', async () => {
        mocked.GET.mockRejectedValueOnce(new ApiClientError(errBody({ status: 401 })))
        const auth = useAuthStore()
        await auth.hydrate()
        expect(auth.status).toBe('anonymous')
    })

    it('hydrate re-throws non-401 errors', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const auth = useAuthStore()
        await expect(auth.hydrate()).rejects.toBeDefined()
    })

    it('refresh updates state with payload', async () => {
        mocked.POST.mockResolvedValueOnce({ data: { data: claims() }, error: undefined })
        const auth = useAuthStore()
        await auth.refresh()
        expect(auth.status).toBe('authenticated')
    })

    it('refresh propagates error from the call', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const auth = useAuthStore()
        await expect(auth.refresh()).rejects.toBeDefined()
    })

    it('logout calls POST, clears state, wipes my-fam-tree:* storage', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: undefined })
        localStorage.setItem('my-fam-tree:foo', 'x')
        localStorage.setItem('unrelated', 'y')
        sessionStorage.setItem('my-fam-tree:bar', 'z')

        const auth = useAuthStore()
        auth.applyClaimsPayload(claims())
        await auth.logout()

        expect(auth.status).toBe('anonymous')
        expect(localStorage.getItem('my-fam-tree:foo')).toBeNull()
        expect(localStorage.getItem('unrelated')).toBe('y')
        expect(sessionStorage.getItem('my-fam-tree:bar')).toBeNull()
    })

    it('logout still clears local state when the POST throws', async () => {
        mocked.POST.mockRejectedValueOnce(new Error('network down'))
        const auth = useAuthStore()
        auth.applyClaimsPayload(claims())
        await auth.logout()
        expect(auth.status).toBe('anonymous')
    })

    it('logout swallows storage failures', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: undefined })
        vi.stubGlobal('sessionStorage', {
            get length(): number {
                throw new Error('safari private')
            },
            getItem: () => null,
            setItem: () => undefined,
            removeItem: () => undefined,
            key: () => null,
        })
        const auth = useAuthStore()
        auth.applyClaimsPayload(claims())
        await expect(auth.logout()).resolves.toBeUndefined()
        expect(auth.status).toBe('anonymous')
    })
})
