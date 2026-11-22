import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

const mutateAsync = vi.fn()
vi.mock('@/api/hooks/auth', () => ({
    useConsumeMagicLink: () => ({ mutateAsync }),
}))

import ConsumeView from '@/views/auth/ConsumeView.vue'
import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'

function makeRouter(): Router {
    return createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/auth/consume', component: { template: '<div />' } },
            { path: '/health', component: { template: '<div />' } },
            { path: '/auth/sign-in', component: { template: '<div />' } },
        ],
    })
}

async function mountConsume(query = 'token=tok') {
    const router = makeRouter()
    await router.push(`/auth/consume?${query}`)
    await router.isReady()
    return mount(ConsumeView, {
        global: {
            plugins: [i18n, router],
            stubs: {
                'v-card': { template: '<div><slot /></div>' },
                'v-progress-circular': { template: '<div class="spinner" />' },
                'v-alert': { template: '<div class="alert"><slot /><slot name="append" /></div>', props: ['type'] },
                'v-btn': { template: '<a class="btn"><slot /></a>', props: ['to', 'variant'] },
            },
        },
    })
}

describe('ConsumeView', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        mutateAsync.mockReset()
        // ConsumeView dedupes via a module-level claim set (claimSingleUseToken)
        // that tests can't clear — so every case uses a UNIQUE token to avoid a
        // prior case's claim short-circuiting it.
    })

    it('shows pending then ok on success', async () => {
        mutateAsync.mockResolvedValueOnce(undefined)
        const w = await mountConsume('token=tok-success')
        await flushPromises()
        expect(w.find('[data-testid="consume-error"]').exists()).toBe(false)
    })

    it('shows error when no token in query', async () => {
        mutateAsync.mockResolvedValueOnce(undefined)
        const w = await mountConsume('')
        await flushPromises()
        expect(w.find('[data-testid="consume-error"]').exists()).toBe(true)
        expect(mutateAsync).not.toHaveBeenCalled()
    })

    it('shows error when consume rejects', async () => {
        mutateAsync.mockRejectedValueOnce(new Error('expired'))
        const w = await mountConsume('token=tok-reject')
        await flushPromises()
        expect(w.find('[data-testid="consume-error"]').exists()).toBe(true)
    })

    it('a second mount of the same token does not re-POST (module dedup)', async () => {
        // First mount consumes the single-use token.
        mutateAsync.mockResolvedValueOnce(undefined)
        await mountConsume('token=tok-double')
        await flushPromises()
        expect(mutateAsync).toHaveBeenCalledTimes(1)
        // A racing re-mount with the SAME token must short-circuit — not fire
        // a second POST that would 401 ("already consumed") and clobber the UI.
        const second = await mountConsume('token=tok-double')
        await flushPromises()
        expect(mutateAsync).toHaveBeenCalledTimes(1)
        expect(second.find('[data-testid="consume-error"]').exists()).toBe(false)
    })

    it('a 401 is benign when a racing fire already authenticated', async () => {
        // Simulate the racing fire's 200 having hydrated the auth store.
        useAuthStore().status = 'authenticated'
        mutateAsync.mockRejectedValueOnce(new Error('already consumed'))
        const w = await mountConsume('token=tok-benign')
        await flushPromises()
        // We're signed in, so the already-consumed 401 must NOT surface the
        // error card — it falls through to the success redirect.
        expect(w.find('[data-testid="consume-error"]').exists()).toBe(false)
    })
})
