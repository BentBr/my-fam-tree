import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

const mutateAsync = vi.fn()
vi.mock('@/api/hooks/owner_transfer', () => ({
    useConfirmOwnerTransfer: () => ({ mutateAsync }),
}))

import { i18n } from '@/i18n'
import OwnerTransferConfirm from '@/views/account/OwnerTransferConfirm.vue'

function makeRouter(): Router {
    return createRouter({
        history: createMemoryHistory(),
        routes: [{ path: '/account/owner-transfer/confirm', component: { template: '<div />' } }],
    })
}

async function mountView(query: string): Promise<ReturnType<typeof mount>> {
    const router = makeRouter()
    await router.push(`/account/owner-transfer/confirm${query.length ? '?' + query : ''}`)
    await router.isReady()
    return mount(OwnerTransferConfirm, {
        global: {
            plugins: [i18n, router],
            stubs: {
                'v-alert': {
                    template: '<div :data-testid="$attrs[\'data-testid\']"><slot /></div>',
                },
            },
        },
    })
}

describe('OwnerTransferConfirm', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        mutateAsync.mockReset()
    })

    it('shows error state when the token query param is missing', async () => {
        const w = await mountView('')
        await flushPromises()
        expect(w.find('[data-testid="owner-transfer-error"]').exists()).toBe(true)
        expect(mutateAsync).not.toHaveBeenCalled()
    })

    it('reports "one side confirmed" when only one side has confirmed so far', async () => {
        mutateAsync.mockResolvedValueOnce({
            from_confirmed: true,
            to_confirmed: false,
        })
        const w = await mountView('token=tok-1')
        await flushPromises()
        expect(mutateAsync).toHaveBeenCalledWith('tok-1')
        expect(w.find('[data-testid="owner-transfer-success-one"]').exists()).toBe(true)
    })

    it('reports "both sides confirmed" when both sides are now confirmed', async () => {
        mutateAsync.mockResolvedValueOnce({
            from_confirmed: true,
            to_confirmed: true,
        })
        const w = await mountView('token=tok-2')
        await flushPromises()
        expect(w.find('[data-testid="owner-transfer-success-both"]').exists()).toBe(true)
    })

    it('shows error state when the confirm mutation rejects', async () => {
        mutateAsync.mockRejectedValueOnce(new Error('bad token'))
        const w = await mountView('token=tok-3')
        await flushPromises()
        expect(w.find('[data-testid="owner-transfer-error"]').exists()).toBe(true)
    })
})
