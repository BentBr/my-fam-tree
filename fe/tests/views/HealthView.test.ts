import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

vi.mock('@/api/hooks/health', () => ({
    useHealth: vi.fn(),
}))

import { useHealth } from '@/api/hooks/health'
import HealthView from '@/views/HealthView.vue'
import { i18n } from '@/i18n'

interface HealthRef {
    data: { value: { data?: { version: string }; meta?: { request_id: string } } | undefined }
    isLoading: { value: boolean }
    error: { value: Error | null }
}

const mocked = useHealth as unknown as ReturnType<typeof vi.fn>

function mountView(state: HealthRef) {
    mocked.mockReturnValueOnce(state)
    return mount(HealthView, {
        global: {
            plugins: [createPinia(), i18n],
            stubs: {
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-card-text': { template: '<div><slot /></div>' },
                'v-progress-linear': { template: '<div class="prog" />' },
                'v-alert': { template: '<div class="alert" :data-type="type"><slot /></div>', props: ['type'] },
                'v-list': { template: '<div><slot /></div>' },
                'v-list-item': { template: '<div class="li" />', props: ['title', 'prependIcon'] },
            },
        },
    })
}

describe('HealthView', () => {
    it('renders the loading state when isLoading is true', () => {
        const w = mountView({
            data: ref(undefined),
            isLoading: ref(true),
            error: ref(null),
        } as never)
        expect(w.find('[data-testid="health-loading"]').exists()).toBe(true)
    })

    it('renders the error state', () => {
        const w = mountView({
            data: ref(undefined),
            isLoading: ref(false),
            error: ref(new Error('boom')),
        } as never)
        expect(w.find('[data-testid="health-error"]').exists()).toBe(true)
    })

    it('renders the success state with version + request id', () => {
        const w = mountView({
            data: ref({ data: { version: '9.9' }, meta: { request_id: 'req-1' } }),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        expect(w.find('[data-testid="health-ok"]').exists()).toBe(true)
    })
})
