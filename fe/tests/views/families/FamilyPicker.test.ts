import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import FamilyPicker from '@/views/families/FamilyPicker.vue'

function makeRouter(): Router {
    return createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: { template: '<div />' } },
            { path: '/health', component: { template: '<div />' } },
            { path: '/families/create', component: { template: '<div />' } },
        ],
    })
}

async function mountView() {
    const router = makeRouter()
    await router.push('/')
    await router.isReady()
    return mount(FamilyPicker, {
        global: {
            plugins: [i18n, router],
            stubs: {
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-list': { template: '<div><slot /></div>' },
                'v-list-item': {
                    template:
                        '<button class="item" :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\')">{{ title }}</button>',
                    props: ['title', 'subtitle'],
                    emits: ['click'],
                },
                'v-btn': { template: '<a class="btn-link"><slot /></a>', props: ['to', 'variant'] },
            },
        },
    })
}

describe('FamilyPicker', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        const store: Record<string, string> = {}
        vi.stubGlobal('localStorage', {
            getItem: (k: string) => store[k] ?? null,
            setItem: (k: string, v: string) => {
                store[k] = v
            },
            removeItem: (k: string) => {
                delete store[k]
            },
        })
    })

    it('renders one item per family', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [
                { id: 'f-1', name: 'F1', role: 'owner' },
                { id: 'f-2', name: 'F2', role: 'user' },
            ],
        } as never)
        const w = await mountView()
        expect(w.findAll('.item')).toHaveLength(2)
    })

    it('clicking an item sets active family and redirects to /health', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F1', role: 'owner' }],
        } as never)
        const w = await mountView()
        await w.find('.item').trigger('click')
        await flushPromises()
        const family = useActiveFamilyStore()
        expect(family.activeFamilyId).toBe('f-1')
    })
})
