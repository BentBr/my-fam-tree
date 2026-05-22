import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'

import ToastContainer from '@/components/common/ToastContainer.vue'
import { i18n } from '@/i18n'
import { useUiStore } from '@/stores/ui'

describe('ToastContainer', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    function mountWithStubs() {
        return mount(ToastContainer, {
            global: {
                plugins: [i18n],
                stubs: {
                    'v-snackbar': {
                        props: ['modelValue', 'color', 'timeout', 'location', 'style'],
                        emits: ['update:modelValue'],
                        template:
                            '<div class="snack-stub" :data-color="color" :data-timeout="timeout" @click="$emit(\'update:modelValue\', false)"><slot /><slot name="actions" /></div>',
                    },
                    'v-btn': { template: '<button class="btn-stub" @click="$emit(\'click\')"><slot /></button>' },
                },
            },
        })
    }

    it('renders nothing when there are no toasts', () => {
        const w = mountWithStubs()
        expect(w.findAll('.snack-stub')).toHaveLength(0)
    })

    it('renders one snackbar per pushed toast', async () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'info', message: 'hi' })
        ui.pushToast({ kind: 'error', message: 'fail', code: 'oops' })
        const w = mountWithStubs()
        expect(w.findAll('.snack-stub')).toHaveLength(2)
    })

    it('uses 8s timeout for errors and 4s for non-errors', async () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'info', message: 'i' })
        ui.pushToast({ kind: 'error', message: 'e' })
        const w = mountWithStubs()
        const snacks = w.findAll('.snack-stub')
        expect(snacks[0]?.attributes('data-timeout')).toBe('4000')
        expect(snacks[1]?.attributes('data-timeout')).toBe('8000')
    })

    it('dismiss button click clears the toast', async () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'info', message: 'gone' })
        const w = mountWithStubs()
        const btn = w.find('.btn-stub')
        await btn.trigger('click')
        expect(ui.toasts).toHaveLength(0)
    })

    it('clicking the snackbar (emits update:model-value false) dismisses', async () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'info', message: 'gone' })
        const w = mountWithStubs()
        const snack = w.find('.snack-stub')
        await snack.trigger('click')
        expect(ui.toasts).toHaveLength(0)
    })
})
