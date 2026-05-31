import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

import type * as FamiliesHooks from '@/api/hooks/families'

interface OverviewShape {
    id: string
    name: string
    role: 'owner' | 'admin' | 'user'
    member_count: number
    person_count: number
    latest_persons: Array<{ id: string; given_name: string; family_name: string; created_at: string }>
}

const overviewData = ref<OverviewShape | undefined>(undefined)
const overviewIsLoading = ref(false)
const overviewError = ref<unknown>(null)
const renameMutate = vi.fn().mockResolvedValue(undefined)

vi.mock('@/api/hooks/families', async (importOriginal) => {
    const actual = await importOriginal<typeof FamiliesHooks>()
    return {
        ...actual,
        useFamilyOverview: () => ({ data: overviewData, isLoading: overviewIsLoading, error: overviewError }),
        useRenameFamily: () => ({ mutateAsync: renameMutate, isPending: ref(false) }),
    }
})

import { i18n } from '@/i18n'
import AdminFamily from '@/views/admin/AdminFamily.vue'

function mountPage() {
    return mount(AdminFamily, {
        global: {
            plugins: [i18n],
            stubs: {
                'v-card': { template: '<div :data-testid="$attrs[\'data-testid\']"><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-card-text': { template: '<div><slot /></div>' },
                'v-skeleton-loader': { template: '<div :data-testid="$attrs[\'data-testid\']" />' },
                'v-alert': { template: '<div :data-testid="$attrs[\'data-testid\']"><slot /></div>' },
                'v-btn': {
                    template:
                        '<button type="button" :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\', $event)"><slot /></button>',
                    emits: ['click'],
                },
                'v-text-field': {
                    template:
                        '<input :value="modelValue" :data-testid="$attrs[\'data-testid\']" @input="$emit(\'update:modelValue\', $event.target.value)" />',
                    props: ['modelValue'],
                    emits: ['update:modelValue'],
                },
                'v-list': { template: '<ul><slot /></ul>' },
                'v-list-item': {
                    template:
                        '<li :data-testid="$attrs[\'data-testid\']" :data-title="title" :data-subtitle="subtitle" :data-to="to" />',
                    props: ['title', 'subtitle', 'to', 'prependIcon'],
                },
            },
        },
    })
}

describe('AdminFamily', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        overviewData.value = undefined
        overviewIsLoading.value = false
        overviewError.value = null
        renameMutate.mockClear()
    })

    it('renders the loading skeleton while the overview query is pending', () => {
        overviewIsLoading.value = true
        const w = mountPage()
        expect(w.find('[data-testid="admin-family-loading"]').exists()).toBe(true)
    })

    it('renders the error alert when the overview query fails', () => {
        overviewError.value = new Error('boom')
        const w = mountPage()
        expect(w.find('[data-testid="admin-family-error"]').exists()).toBe(true)
    })

    it('renders name, member count, person count and the latest-3 list when loaded', () => {
        overviewData.value = {
            id: 'fam-1',
            name: 'Mustermanns',
            role: 'owner',
            member_count: 4,
            person_count: 27,
            latest_persons: [
                { id: 'p-1', given_name: 'Ada', family_name: 'Lovelace', created_at: '2026-05-30T10:00:00Z' },
                { id: 'p-2', given_name: 'Alan', family_name: 'Turing', created_at: '2026-05-29T10:00:00Z' },
            ],
        }
        const w = mountPage()
        expect(w.find('[data-testid="admin-family-name"]').text()).toBe('Mustermanns')
        expect(w.find('[data-testid="admin-family-member-count"]').text()).toBe('4')
        expect(w.find('[data-testid="admin-family-person-count"]').text()).toBe('27')
        // Two latest-person rows, each linking to /tree?center=<id>.
        const adaRow = w.find('[data-testid="admin-family-latest-p-1"]')
        expect(adaRow.exists()).toBe(true)
        expect(adaRow.attributes('data-title')).toBe('Ada Lovelace')
        expect(adaRow.attributes('data-to')).toBe('/tree?center=p-1')
    })

    it('shows the empty-list message when no persons have been added yet', () => {
        overviewData.value = {
            id: 'fam-1',
            name: 'Empty Fam',
            role: 'owner',
            member_count: 1,
            person_count: 0,
            latest_persons: [],
        }
        const w = mountPage()
        expect(w.find('[data-testid="admin-family-latest-empty"]').exists()).toBe(true)
    })

    it('switches to the inline edit field when Rename is clicked, then commits on Save', async () => {
        overviewData.value = {
            id: 'fam-1',
            name: 'Old Name',
            role: 'owner',
            member_count: 1,
            person_count: 0,
            latest_persons: [],
        }
        const w = mountPage()
        await w.find('[data-testid="admin-family-rename"]').trigger('click')
        await flushPromises()
        const input = w.find('[data-testid="admin-family-name-input"]')
        expect(input.exists()).toBe(true)
        await input.setValue('New Name')
        await w.find('[data-testid="admin-family-name-save"]').trigger('click')
        await flushPromises()
        expect(renameMutate).toHaveBeenCalledWith({ id: 'fam-1', name: 'New Name' })
    })

    it('cancels rename without calling the mutation', async () => {
        overviewData.value = {
            id: 'fam-1',
            name: 'Stay',
            role: 'owner',
            member_count: 1,
            person_count: 0,
            latest_persons: [],
        }
        const w = mountPage()
        await w.find('[data-testid="admin-family-rename"]').trigger('click')
        await flushPromises()
        await w.find('[data-testid="admin-family-name-input"]').setValue('Drafted-but-cancelled')
        await w.find('[data-testid="admin-family-name-cancel"]').trigger('click')
        await flushPromises()
        expect(renameMutate).not.toHaveBeenCalled()
        // Back in view mode → static name still rendered.
        expect(w.find('[data-testid="admin-family-name"]').exists()).toBe(true)
    })
})
