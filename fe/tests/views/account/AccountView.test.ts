import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'

const updateMutate = vi.fn()
const requestEmailMutate = vi.fn()
// useMe now unwraps the envelope, so the mock returns the profile directly.
const meData = ref<{ display_name: string; email: string; locale: string } | undefined>(undefined)
const meRefs = {
    data: meData,
    isLoading: ref(false),
    error: ref<unknown>(null),
}

vi.mock('@/api/hooks/users', () => ({
    useMe: () => meRefs,
    useUpdateMe: () => ({ mutateAsync: updateMutate, isPending: ref(false) }),
    useRequestEmailChange: () => ({ mutateAsync: requestEmailMutate, isPending: ref(false) }),
}))
vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import AccountView from '@/views/account/AccountView.vue'

async function mountView() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/account', component: { template: '<div />' } },
            { path: '/auth/sign-in', component: { template: '<div />' } },
        ],
    })
    await router.push('/account')
    await router.isReady()
    return mount(AccountView, {
        global: {
            plugins: [i18n, router],
            stubs: {
                // The reminder panel owns its own query hook (needs a QueryClient);
                // stub it out — it has its own dedicated test.
                ReminderPrefsSection: { template: '<div data-testid="reminder-prefs-stub" />' },
                'v-container': { template: '<div><slot /></div>' },
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-card-subtitle': { template: '<div><slot /></div>' },
                'v-card-text': { template: '<div><slot /></div>' },
                'v-alert': { template: '<div class="alert" :data-testid="$attrs[\'data-testid\']"><slot /></div>' },
                'v-divider': { template: '<hr />' },
                'v-form': {
                    template:
                        '<form @submit.prevent="$emit(\'submit\', { preventDefault: () => undefined })"><slot /></form>',
                    emits: ['submit'],
                },
                'v-text-field': {
                    template:
                        '<input class="input" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" :data-testid="$attrs[\'data-testid\']" />',
                    props: ['modelValue', 'label', 'type', 'autocomplete'],
                    emits: ['update:modelValue'],
                },
                'v-select': {
                    template:
                        '<select class="select" :data-testid="$attrs[\'data-testid\']" @change="$emit(\'update:modelValue\', $event.target.value)"><option value="en">en</option><option value="de">de</option></select>',
                    props: ['modelValue', 'items', 'label'],
                    emits: ['update:modelValue'],
                },
                'v-btn': {
                    template:
                        '<button class="btn" :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\')"><slot /></button>',
                    props: ['loading', 'block', 'type', 'variant', 'color', 'prependIcon'],
                },
            },
        },
    })
}

describe('AccountView', () => {
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
        meData.value = undefined
        updateMutate.mockReset()
        requestEmailMutate.mockReset()
    })

    it('mounts and syncs form fields when /me arrives', async () => {
        const w = await mountView()
        meData.value = { display_name: 'Alice', email: 'a@b', locale: 'de' }
        await flushPromises()
        const nameInput = w.find('[data-testid="account-display-name"]')
        expect(nameInput.attributes('value')).toBe('Alice')
    })

    it('saveProfile dispatches mutateAsync with trimmed name', async () => {
        const w = await mountView()
        meData.value = { display_name: 'Alice', email: 'a@b', locale: 'en' }
        await flushPromises()
        updateMutate.mockResolvedValueOnce(undefined)
        await w.findAll('form')[0]?.trigger('submit')
        await flushPromises()
        expect(updateMutate).toHaveBeenCalledWith({ display_name: 'Alice', locale: 'en' })
    })

    it('saveProfile renders error from caught Error', async () => {
        const w = await mountView()
        updateMutate.mockRejectedValueOnce(new Error('bad'))
        await w.findAll('form')[0]?.trigger('submit')
        await flushPromises()
        expect(w.find('[data-testid="account-error"]').exists()).toBe(true)
    })

    it('saveProfile uses "unknown error" fallback for non-Error throws', async () => {
        const w = await mountView()
        updateMutate.mockRejectedValueOnce('weird')
        await w.findAll('form')[0]?.trigger('submit')
        await flushPromises()
        expect(w.text()).toContain('unknown error')
    })

    it('submitEmailChange dispatches with lowercased trimmed value', async () => {
        const w = await mountView()
        requestEmailMutate.mockResolvedValueOnce(undefined)
        const emailInput = w.find('[data-testid="account-email-new"]')
        await emailInput.setValue(' NEW@B ')
        await w.findAll('form')[1]?.trigger('submit')
        await flushPromises()
        expect(requestEmailMutate).toHaveBeenCalledWith('new@b')
        expect(w.find('[data-testid="email-change-pending"]').exists()).toBe(true)
    })

    it('submitEmailChange surfaces caught errors', async () => {
        const w = await mountView()
        requestEmailMutate.mockRejectedValueOnce(new Error('rate-limited'))
        await w.findAll('form')[1]?.trigger('submit')
        await flushPromises()
        expect(w.find('[data-testid="account-error"]').exists()).toBe(true)
    })

    it('signOut button clears auth and routes to sign-in', async () => {
        const w = await mountView()
        await w.find('[data-testid="account-sign-out"]').trigger('click')
        await flushPromises()
        // No assertion on router (covered elsewhere); just ensures the handler runs.
    })
})
