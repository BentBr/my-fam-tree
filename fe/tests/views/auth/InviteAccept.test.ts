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
                'v-alert': {
                    // Render both the default slot AND the `#append` slot
                    // (the mismatch sign-out button lives in `#append`).
                    template: '<div><slot /><slot name="append" /></div>',
                    props: ['type'],
                },
                'v-btn': {
                    template: '<button v-bind="$attrs" @click="$emit(\'click\')"><slot /></button>',
                    props: ['variant'],
                    inheritAttrs: false,
                },
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

    it('anonymous user posts the token directly (no sessionStorage stash)', async () => {
        // The invite token IS the auth factor on the BE — anonymous
        // callers go through the same `mutateAsync` path; the BE
        // find-or-creates the user and issues a cookie inline.
        mutateAsync.mockResolvedValueOnce({ data: { family: { id: 'f-1', name: 'F' } } })
        await mountInvite('token=tok-1')
        await flushPromises()
        expect(mutateAsync).toHaveBeenCalledWith('tok-1')
        // Critically: no token is stashed in sessionStorage (avoiding the
        // earlier security-smell pattern).
        expect(sessionStorage.getItem('my-family:inviteToken')).toBeNull()
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

    it('renders mismatch alert when accept fails with invite_email_mismatch violation', async () => {
        // 422 Validation body: the FE reads `body.fields[].code` for the
        // field-level violation. The top-level code is `validation_failed`.
        const validationError = Object.assign(new Error('mismatch'), {
            code: 'validation_failed',
            body: {
                code: 'validation_failed',
                fields: [{ code: 'validation.invite_email_mismatch', path: '/token' }],
            },
        })
        mutateAsync.mockRejectedValueOnce(validationError)
        const w = await mountInvite('token=tok')
        // Two flushes: one for the mutateAsync rejection to settle, one
        // for the resulting `status.value = 'mismatch'` to re-render.
        await flushPromises()
        await flushPromises()
        expect(w.find('[data-testid="invite-mismatch"]').exists()).toBe(true)
        expect(w.find('[data-testid="invite-mismatch-signout"]').exists()).toBe(true)
        // Confidence: the regular error alert is NOT also rendered.
        expect(w.find('[data-testid="invite-error"]').exists()).toBe(false)
    })

    it('signOutAndRetry clears the session and bounces back to /invite/accept', async () => {
        // First mount lands in the mismatch state.
        const validationError = Object.assign(new Error('mismatch'), {
            body: { fields: [{ code: 'validation.invite_email_mismatch' }] },
        })
        mutateAsync.mockRejectedValueOnce(validationError)
        const w = await mountInvite('token=tok-x')
        await flushPromises()
        await flushPromises()

        // Spy on auth.logout so we don't actually hit the network.
        const auth = useAuthStore()
        const logoutSpy = vi.spyOn(auth, 'logout').mockResolvedValue()

        // Set up the second mount path: a new mutateAsync result so the
        // bounced visit can fall through to the anonymous-accept arm.
        mutateAsync.mockResolvedValueOnce({ data: { family: { id: 'f-1', name: 'F' } } })

        await w.find('[data-testid="invite-mismatch-signout"]').trigger('click')
        await flushPromises()

        expect(logoutSpy).toHaveBeenCalled()
    })
})
