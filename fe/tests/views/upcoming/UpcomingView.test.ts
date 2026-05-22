import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

vi.mock('@/api/hooks/upcoming', () => ({
    useUpcoming: vi.fn(),
}))

vi.mock('vue-router', () => ({
    useRouter: () => ({ push: routerPush }),
}))

import { useUpcoming } from '@/api/hooks/upcoming'
import UpcomingView from '@/views/upcoming/UpcomingView.vue'
import { i18n } from '@/i18n'

const routerPush = vi.fn()

interface QueryState {
    data: { value: unknown }
    isLoading: { value: boolean }
    error: { value: Error | null }
}

const mocked = useUpcoming as unknown as ReturnType<typeof vi.fn>

function stubs() {
    return {
        'v-toolbar': { template: '<div><slot /></div>' },
        'v-toolbar-title': { template: '<div><slot /></div>' },
        'v-spacer': { template: '<div />' },
        // v-btn-toggle accepts v-model + emits 'update:modelValue'; mirror that
        // so the test can observe filter changes via the toggle calls.
        'v-btn-toggle': {
            template: '<div data-testid="upcoming-filter"><slot /></div>',
            props: ['modelValue'],
            emits: ['update:modelValue'],
        },
        'v-btn': {
            template: '<button :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\', $event)"><slot /></button>',
            inheritAttrs: false,
            emits: ['click'],
        },
        'v-skeleton-loader': { template: '<div class="skel" />' },
        'v-alert': { template: '<div class="alert" :data-testid="$attrs[\'data-testid\']"><slot /></div>', inheritAttrs: false },
        'v-card': { template: '<div><slot /></div>' },
        'v-card-title': { template: '<div><slot /></div>' },
        'v-card-text': { template: '<div><slot /></div>' },
        'v-list': { template: '<ul :data-testid="$attrs[\'data-testid\']"><slot /></ul>', inheritAttrs: false },
        'v-list-item': {
            template:
                '<li :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\', $event)"><slot /><slot name="append" /></li>',
            inheritAttrs: false,
            emits: ['click'],
        },
        'v-list-item-title': { template: '<div class="title"><slot /></div>' },
        'v-list-item-subtitle': { template: '<div class="sub"><slot /></div>' },
        'v-chip': { template: '<span class="chip" :data-testid="$attrs[\'data-testid\']"><slot /></span>', inheritAttrs: false },
    }
}

function mountView(state: QueryState) {
    mocked.mockReturnValueOnce(state)
    return mount(UpcomingView, {
        global: {
            plugins: [createPinia(), i18n],
            stubs: stubs(),
        },
    })
}

describe('UpcomingView', () => {
    it('renders rows from the upcoming query', () => {
        const w = mountView({
            data: ref([
                {
                    kind: 'birthday',
                    next_date: '2026-12-01',
                    years: 30,
                    person_id: 'p1',
                    partnership_id: null,
                    label: 'Alice — 30th birthday',
                },
                {
                    kind: 'wedding_anniversary',
                    next_date: '2027-01-15',
                    years: 5,
                    person_id: null,
                    partnership_id: 'pa1',
                    label: 'Alice & Bob — 5th anniversary',
                },
            ]),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        expect(w.find('[data-testid="upcoming-list"]').exists()).toBe(true)
        expect(w.find('[data-testid="upcoming-row-birthday"]').exists()).toBe(true)
        expect(w.find('[data-testid="upcoming-row-wedding_anniversary"]').exists()).toBe(true)
        expect(w.text()).toContain('Alice — 30th birthday')
    })

    it('renders the empty state when the query returns no rows', () => {
        const w = mountView({
            data: ref([]),
            isLoading: ref(false),
            error: ref(null),
        } as never)
        expect(w.find('[data-testid="upcoming-empty"]').exists()).toBe(true)
    })

    it('renders the error state when the query errors', () => {
        const w = mountView({
            data: ref(undefined),
            isLoading: ref(false),
            error: ref(new Error('boom')),
        } as never)
        expect(w.find('[data-testid="upcoming-error"]').exists()).toBe(true)
    })

    it('toggling birthday once enters filter=birthday; clicking again reverts to all', async () => {
        // The view re-calls useUpcoming on each render with the same ref.
        // Returning a captured ref lets us watch the filter value change.
        const data = ref<unknown>([])
        const stateFactory = (): QueryState => ({
            data: data as never,
            isLoading: ref(false),
            error: ref(null),
        })
        // Always provide the stateFactory output when the view re-mounts the hook.
        mocked.mockImplementation((filterRef: { value: string }) => {
            // Mirror filter to data for assertion convenience.
            data.value = filterRef
            return stateFactory()
        })

        const w = mount(UpcomingView, {
            global: { plugins: [createPinia(), i18n], stubs: stubs() },
        })

        // Initial: filter ref starts at 'all'.
        const filterRefAfterMount = data.value as { value: string }
        expect(filterRefAfterMount.value).toBe('all')

        // First click on Birthday ⇒ filter becomes 'birthday'.
        await w.find('[data-testid="upcoming-filter-birthday"]').trigger('click')
        expect((data.value as { value: string }).value).toBe('birthday')

        // Second click on Birthday ⇒ filter reverts to 'all'.
        await w.find('[data-testid="upcoming-filter-birthday"]').trigger('click')
        expect((data.value as { value: string }).value).toBe('all')

        // Click on Anniversary ⇒ filter becomes 'anniversary'.
        await w.find('[data-testid="upcoming-filter-anniversary"]').trigger('click')
        expect((data.value as { value: string }).value).toBe('anniversary')
    })
})
