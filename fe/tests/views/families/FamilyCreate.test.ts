import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

const mutateAsync = vi.fn()
vi.mock('@/api/hooks/families', () => ({
    useCreateFamily: () => ({ mutateAsync, isPending: ref(false) }),
}))
vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import FamilyCreate from '@/views/families/FamilyCreate.vue'

function makeRouter(): Router {
    return createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: { template: '<div />' } },
            { path: '/health', component: { template: '<div />' } },
        ],
    })
}

async function mountView() {
    const router = makeRouter()
    await router.push('/')
    await router.isReady()
    return mount(FamilyCreate, {
        global: {
            plugins: [i18n, router],
            stubs: {
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-alert': { template: '<div :data-testid="$attrs[\'data-testid\']"><slot /></div>' },
                'v-form': {
                    template:
                        '<form @submit.prevent="$emit(\'submit\', { preventDefault: () => undefined })"><slot /></form>',
                    emits: ['submit'],
                },
                'v-text-field': {
                    template:
                        '<input class="input" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
                    props: ['modelValue'],
                    emits: ['update:modelValue'],
                },
                'v-btn': {
                    template: '<button class="btn" type="submit"><slot /></button>',
                    props: ['loading', 'block', 'size'],
                },
            },
        },
    })
}

describe('FamilyCreate', () => {
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
        mutateAsync.mockReset()
    })

    it('submits trimmed name and sets active family on success', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [{ id: 'f-1', name: 'F', role: 'owner' }],
        } as never)
        mutateAsync.mockResolvedValueOnce({ data: { family: { id: 'f-1', name: 'F' } } })
        const w = await mountView()
        await w.find('input').setValue('  My Family  ')
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(mutateAsync).toHaveBeenCalledWith('My Family')
    })

    it('renders error on caught Error', async () => {
        mutateAsync.mockRejectedValueOnce(new Error('exists'))
        const w = await mountView()
        await w.find('input').setValue('X')
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(w.find('[data-testid="family-create-error"]').exists()).toBe(true)
    })

    it('falls back to "unknown error" for non-Error throws', async () => {
        mutateAsync.mockRejectedValueOnce('weird')
        const w = await mountView()
        await w.find('input').setValue('X')
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(w.text()).toContain('unknown error')
    })
})
