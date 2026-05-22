import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const mutateAsync = vi.fn()
vi.mock('@/api/hooks/auth', () => ({
    useRequestMagicLink: () => ({ mutateAsync, isPending: { value: false } }),
}))

import { i18n } from '@/i18n'
import LoginView from '@/views/auth/LoginView.vue'

function mountView() {
    return mount(LoginView, {
        global: {
            plugins: [i18n],
            stubs: {
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-card-subtitle': { template: '<div><slot /></div>' },
                'v-alert': { template: '<div class="alert"><slot /></div>', props: ['type'] },
                'v-form': {
                    template:
                        '<form @submit.prevent="$emit(\'submit\', { preventDefault: () => undefined })"><slot /></form>',
                    emits: ['submit'],
                },
                'v-text-field': {
                    template:
                        '<input class="input" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
                    props: ['modelValue', 'label'],
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

describe('LoginView', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        mutateAsync.mockReset()
    })

    it('submits the email and shows the success card', async () => {
        mutateAsync.mockResolvedValueOnce(undefined)
        const w = mountView()
        await w.find('input').setValue('a@b')
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(mutateAsync).toHaveBeenCalledWith('a@b')
        expect(w.find('[data-testid="sign-in-sent"]').exists()).toBe(true)
    })

    it('renders the error alert on submit failure', async () => {
        mutateAsync.mockRejectedValueOnce(new Error('rate-limited'))
        const w = mountView()
        await w.find('input').setValue('a@b')
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(w.find('[data-testid="login-error"]').exists()).toBe(true)
        expect(w.text()).toContain('rate-limited')
    })

    it('uses fallback error message when caught value is not an Error', async () => {
        mutateAsync.mockRejectedValueOnce('plain-string')
        const w = mountView()
        await w.find('input').setValue('a@b')
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(w.text()).toContain('unknown error')
    })
})
