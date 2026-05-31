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

interface HealthData {
    version: string
    db_ok?: boolean
    db_latency_ms?: number
    server_duration_ms?: number
    worker_ok?: boolean
}
interface HealthRef {
    data: { value: { data?: HealthData; meta?: { request_id: string } } | undefined }
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
                'v-chip': {
                    template: '<div class="chip" :data-color="color"><slot /></div>',
                    props: ['color', 'variant'],
                },
                'v-icon': { template: '<i />', props: ['icon', 'start'] },
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

    it('renders the success state with a green DB chip for fast latency', () => {
        const w = mountView({
            data: ref({
                data: { version: '9.9', db_ok: true, db_latency_ms: 42, server_duration_ms: 55 },
                meta: { request_id: 'req-1' },
            }),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        expect(w.find('[data-testid="health-ok"]').exists()).toBe(true)
        expect(w.find('[data-testid="health-db"]').attributes('data-color')).toBe('success')
    })

    it('colours the DB chip yellow for borderline latency and red when slow', () => {
        const yellow = mountView({
            data: ref({
                data: { version: '9.9', db_ok: true, db_latency_ms: 150, server_duration_ms: 160 },
                meta: {},
            }),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        expect(yellow.find('[data-testid="health-db"]').attributes('data-color')).toBe('warning')

        const red = mountView({
            data: ref({
                data: { version: '9.9', db_ok: true, db_latency_ms: 250, server_duration_ms: 260 },
                meta: {},
            }),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        expect(red.find('[data-testid="health-db"]').attributes('data-color')).toBe('error')
    })

    it('shows an unreachable DB as red', () => {
        const w = mountView({
            data: ref({
                data: { version: '9.9', db_ok: false, db_latency_ms: 5, server_duration_ms: 12 },
                meta: {},
            }),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        expect(w.find('[data-testid="health-db"]').attributes('data-color')).toBe('error')
    })

    it('shows the server-latency chip alongside the DB chip with its own threshold colour', () => {
        // server_duration_ms uses the same three-tier scale as the DB
        // chip but is independent: a fast DB + slow API still flags
        // red on the server chip without dragging the DB chip down.
        const w = mountView({
            data: ref({
                data: { version: '9.9', db_ok: true, db_latency_ms: 5, server_duration_ms: 250 },
                meta: {},
            }),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        expect(w.find('[data-testid="health-db"]').attributes('data-color')).toBe('success')
        expect(w.find('[data-testid="health-server"]').attributes('data-color')).toBe('error')
    })

    it('renders the worker chip green when worker_ok is true', () => {
        // The worker chip is a pure boolean — green when the worker is
        // currently holding the Redis leader lease, red when not. No
        // latency dimension to colour-grade.
        const w = mountView({
            data: ref({
                data: {
                    version: '9.9',
                    db_ok: true,
                    db_latency_ms: 5,
                    server_duration_ms: 12,
                    worker_ok: true,
                },
                meta: {},
            }),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        const worker = w.find('[data-testid="health-worker"]')
        expect(worker.exists()).toBe(true)
        expect(worker.attributes('data-color')).toBe('success')
        expect(worker.text()).toContain('Worker alive')
    })

    it('renders the worker chip red when worker_ok is false', () => {
        // worker_ok flips to false within ~30 s of the worker crashing
        // (its Redis lease expires). The chip should reflect that
        // immediately and the user-facing label should say "down".
        const w = mountView({
            data: ref({
                data: {
                    version: '9.9',
                    db_ok: true,
                    db_latency_ms: 5,
                    server_duration_ms: 12,
                    worker_ok: false,
                },
                meta: {},
            }),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        const worker = w.find('[data-testid="health-worker"]')
        expect(worker.attributes('data-color')).toBe('error')
        expect(worker.text()).toContain('Worker down')
    })
})
