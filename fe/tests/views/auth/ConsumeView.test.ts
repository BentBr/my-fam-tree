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
        // ConsumeView dedupes by token in sessionStorage; tests share the
        // global, so wipe between cases or a previous "succeeded for tok"
        // would short-circuit the next "rejects for tok".
        sessionStorage.clear()
    })

    it('shows pending then ok on success', async () => {
        mutateAsync.mockResolvedValueOnce(undefined)
        const w = await mountConsume()
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
        const w = await mountConsume()
        await flushPromises()
        expect(w.find('[data-testid="consume-error"]').exists()).toBe(true)
    })
})
