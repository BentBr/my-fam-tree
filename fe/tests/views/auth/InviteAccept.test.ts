import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

const mutateAsync = vi.fn()
vi.mock('@/api/hooks/families', () => ({
    useAcceptInvite: () => ({ mutateAsync }),
}))
vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import InviteAccept from '@/views/auth/InviteAccept.vue'

function mockStorage(): void {
    const local: Record<string, string> = {}
    const session: Record<string, string> = {}
    vi.stubGlobal('localStorage', {
        getItem: (k: string) => local[k] ?? null,
        setItem: (k: string, v: string) => {
            local[k] = v
        },
        removeItem: (k: string) => {
            delete local[k]
        },
    })
    vi.stubGlobal('sessionStorage', {
        getItem: (k: string) => session[k] ?? null,
        setItem: (k: string, v: string) => {
            session[k] = v
        },
        removeItem: (k: string) => {
            delete session[k]
        },
    })
}

function makeRouter(): Router {
    return createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/invite/accept', component: { template: '<div />' } },
            { path: '/health', component: { template: '<div />' } },
            { path: '/auth/sign-in', component: { template: '<div />' } },
        ],
    })
}

async function mountInvite(query: string): Promise<ReturnType<typeof mount>> {
    const router = makeRouter()
    await router.push(`/invite/accept${query.length ? '?' + query : ''}`)
    await router.isReady()
    return mount(InviteAccept, {
        global: {
            plugins: [i18n, router],
            stubs: {
                'v-card': { template: '<div><slot /></div>' },
                'v-progress-circular': { template: '<div />' },
                'v-alert': { template: '<div><slot /></div>', props: ['type'] },
            },
        },
    })
}

describe('InviteAccept', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        mockStorage()
        mutateAsync.mockReset()
    })

    it('error state when token is missing', async () => {
        const w = await mountInvite('')
        await flushPromises()
        expect(w.find('[data-testid="invite-error"]').exists()).toBe(true)
        expect(mutateAsync).not.toHaveBeenCalled()
    })

    it('anonymous user gets bounced to sign-in with token stashed', async () => {
        await mountInvite('token=tok-1')
        await flushPromises()
        expect(sessionStorage.getItem('my-family:inviteToken')).toBe('tok-1')
    })

    it('authenticated user calls accept and sets active family', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        mutateAsync.mockResolvedValueOnce({ data: { family: { id: 'f-1', name: 'F' } } })
        await mountInvite('token=tok')
        await flushPromises()
        expect(mutateAsync).toHaveBeenCalledWith('tok')
    })

    it('authenticated user sees error when accept rejects', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [],
        } as never)
        mutateAsync.mockRejectedValueOnce(new Error('bad'))
        const w = await mountInvite('token=tok')
        await flushPromises()
        expect(w.find('[data-testid="invite-error"]').exists()).toBe(true)
    })
})
