import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

interface Person {
    id: string
    given_name: string
    family_name: string
    notes?: string
    gender?: string
    birth_date?: string | null
    birth_place?: string
    death_date?: string | null
    nickname?: string
}

const listData = ref<Person[] | undefined>([
    { id: 'p1', given_name: 'A', family_name: 'X', notes: 'note' },
    { id: 'p2', given_name: 'B', family_name: 'Y' },
])
const delMutate = vi.fn()
const addParentMutate = vi.fn()
const partnerMutate = vi.fn()

vi.mock('@/api/hooks/persons', () => ({
    useListPersons: () => ({ data: listData, isLoading: ref(false), error: ref(null) }),
    useDeletePerson: () => ({ mutateAsync: delMutate, isPending: ref(false) }),
}))
vi.mock('@/api/hooks/relationships', () => ({
    useAddParentLink: () => ({ mutateAsync: addParentMutate, isPending: ref(false) }),
    useCreatePartnership: () => ({ mutateAsync: partnerMutate, isPending: ref(false) }),
}))

import { i18n } from '@/i18n'
import PersonDetail from '@/views/tree/PersonDetail.vue'

function mountDetail(personId: string) {
    return mount(PersonDetail, {
        props: { personId },
        global: {
            plugins: [createPinia(), i18n],
            stubs: {
                'v-list': { template: '<div><slot /></div>' },
                'v-list-item': { template: '<div><slot /></div>' },
                'v-list-item-title': { template: '<div><slot /></div>' },
                'v-list-item-subtitle': { template: '<div><slot /></div>' },
                'v-divider': { template: '<hr />' },
                'v-select': {
                    // Stub renders one <option> per item so `setValue` works for
                    // any value the test passes in (people ids, parent kinds,
                    // partner kinds). Mirrors how the real v-select binds.
                    template: `
                        <select
                            :data-testid="$attrs['data-testid']"
                            @change="$emit('update:modelValue', $event.target.value)"
                        >
                            <option v-for="item in items" :key="item.value ?? item" :value="item.value ?? item">
                                {{ item.title ?? item }}
                            </option>
                        </select>
                    `,
                    props: ['modelValue', 'items', 'label'],
                    emits: ['update:modelValue'],
                },
                'v-btn': {
                    template:
                        '<button :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\')"><slot /></button>',
                    props: ['loading', 'block', 'color', 'variant', 'disabled'],
                    emits: ['click'],
                },
                'v-dialog': {
                    template: '<div v-if="modelValue"><slot /></div>',
                    props: ['modelValue', 'maxWidth'],
                },
                'v-card': { template: '<div><slot /></div>' },
                'v-card-title': { template: '<div><slot /></div>' },
                'v-card-text': { template: '<div><slot /></div>' },
                'v-card-actions': { template: '<div><slot /></div>' },
                'v-spacer': { template: '<div />' },
                PersonEdit: {
                    template: '<div class="edit-stub" @click="$emit(\'saved\')" />',
                    emits: ['saved', 'cancel'],
                    props: ['mode', 'initial'],
                },
            },
        },
    })
}

describe('PersonDetail', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        delMutate.mockReset()
        addParentMutate.mockReset()
        partnerMutate.mockReset()
        listData.value = [
            { id: 'p1', given_name: 'A', family_name: 'X', notes: 'note' },
            { id: 'p2', given_name: 'B', family_name: 'Y' },
        ]
    })

    it('renders given+family name and notes', () => {
        const w = mountDetail('p1')
        expect(w.text()).toContain('A')
        expect(w.text()).toContain('X')
    })

    it('renders nothing-but-close when person not found', () => {
        listData.value = []
        const w = mountDetail('ghost')
        expect(w.find('[data-testid="person-detail-close"]').exists()).toBe(true)
    })

    it('emits close when the close button is clicked', async () => {
        const w = mountDetail('p1')
        await w.find('[data-testid="person-detail-close"]').trigger('click')
        expect(w.emitted('close')).toBeDefined()
    })

    it('linkParent calls mutation when a parent is selected', async () => {
        addParentMutate.mockResolvedValueOnce(undefined)
        const w = mountDetail('p1')
        await w.find('[data-testid="person-add-parent"]').setValue('p2')
        await w.find('[data-testid="person-add-parent-submit"]').trigger('click')
        await flushPromises()
        expect(addParentMutate).toHaveBeenCalledWith({
            child_id: 'p1',
            parent_id: 'p2',
            kind: 'biological',
        })
    })

    it('linkPartner calls mutation when both a partner and kind are selected', async () => {
        // The partner-kind dropdown has no default — submit stays disabled
        // until the user picks marriage / civil_union / partnership. The
        // test must therefore set both fields before clicking submit, and
        // assert the chosen kind reaches the mutation.
        partnerMutate.mockResolvedValueOnce(undefined)
        const w = mountDetail('p1')
        await w.find('[data-testid="person-add-partner"]').setValue('p2')
        await w.find('[data-testid="person-add-partner-kind"]').setValue('marriage')
        await w.find('[data-testid="person-add-partner-submit"]').trigger('click')
        await flushPromises()
        expect(partnerMutate).toHaveBeenCalledWith({
            partner_a_id: 'p1',
            partner_b_id: 'p2',
            kind: 'marriage',
        })
    })

    it('delete confirm dispatches mutation and emits changed + close', async () => {
        delMutate.mockResolvedValueOnce(undefined)
        const w = mountDetail('p1')
        await w.find('[data-testid="person-delete-button"]').trigger('click')
        await w.find('[data-testid="person-delete-confirm"]').trigger('click')
        await flushPromises()
        expect(delMutate).toHaveBeenCalledWith('p1')
        expect(w.emitted('changed')).toBeDefined()
        expect(w.emitted('close')).toBeDefined()
    })

    it('switches to edit mode + back via PersonEdit saved', async () => {
        const w = mountDetail('p1')
        await w.find('[data-testid="person-edit-button"]').trigger('click')
        expect(w.find('.edit-stub').exists()).toBe(true)
        await w.find('.edit-stub').trigger('click') // emit saved
        await flushPromises()
        expect(w.emitted('changed')).toBeDefined()
    })
})
