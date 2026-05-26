import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

interface Person {
    id: string
    given_name: string
    family_name: string
    name_at_birth?: string
    notes?: string
    gender?: string
    birth_date?: string | null
    birth_place?: string
    death_date?: string | null
    nickname?: string
    linked_user_id?: string | null
}

const listData = ref<Person[] | undefined>([
    {
        id: 'p1',
        given_name: 'A',
        family_name: 'X',
        notes: 'note',
        birth_place: 'Hamburg',
        nickname: 'Ay',
        gender: 'male',
        birth_date: '1965-04-22',
        name_at_birth: '',
        death_date: null,
    },
    { id: 'p2', given_name: 'B', family_name: 'Y', birth_place: '', nickname: '', gender: '', name_at_birth: '' },
])
const personGetData = ref<Person | undefined>(undefined)
const personGetIsLoading = ref(false)

const delMutate = vi.fn()
const addParentMutate = vi.fn()
const partnerMutate = vi.fn()
const updatePartnerMutate = vi.fn()
const deletePartnerMutate = vi.fn()
const deleteParentMutate = vi.fn()

// Simulate the seeded relations: Klaus (`p1`) has parents p_parent_a and
// p_parent_b. Klaus↔p2 is a partnership. Used by the partners-panel tests.
const treeData = ref({
    nodes: [],
    parent_edges: [
        { a: 'p1', b: 'p_parent_a', kind: 'biological' },
        { a: 'p1', b: 'p_parent_b', kind: 'biological' },
    ],
    partner_edges: [
        {
            id: 'part-1',
            a: 'p1',
            b: 'p2',
            kind: 'civil_union',
            started_on: '1990-01-01',
            ended_on: null,
            end_reason: null,
        },
    ],
})

// Pinia store state — flipped between tests via `activeRole`.
const activeRole = ref<'owner' | 'admin' | 'user' | null>('owner')
// Signed-in user id — drives the user-role self-edit gate. Defaults to a
// value distinct from the persons in `listData` so tests that don't care
// about self-edit don't accidentally grant edit rights to user role.
const currentUserId = ref<string>('u-self')

vi.mock('@/stores/activeFamily', () => ({
    useActiveFamilyStore: () => ({
        activeFamily: { id: 'fam', name: 'Müller', role: activeRole.value },
    }),
}))

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        user: { id: currentUserId.value, email: 'a@b.c', locale: 'en', displayName: '' },
    }),
}))

vi.mock('@/api/hooks/persons', () => ({
    useListPersons: () => ({ data: listData, isLoading: ref(false), error: ref(null) }),
    useGetPerson: () => ({ data: personGetData, isLoading: personGetIsLoading, error: ref(null) }),
    useDeletePerson: () => ({ mutateAsync: delMutate, isPending: ref(false) }),
}))

const inviteMutate = vi.fn()
vi.mock('@/api/hooks/invites', () => ({
    useCreateInvite: () => ({ mutateAsync: inviteMutate, isPending: ref(false) }),
}))

vi.mock('@/api/hooks/relationships', () => ({
    useTree: () => ({ data: treeData, isLoading: ref(false), error: ref(null) }),
    useAddParentLink: () => ({ mutateAsync: addParentMutate, isPending: ref(false) }),
    useCreatePartnership: () => ({ mutateAsync: partnerMutate, isPending: ref(false) }),
    useUpdatePartnership: () => ({ mutateAsync: updatePartnerMutate, isPending: ref(false) }),
    useDeletePartnership: () => ({ mutateAsync: deletePartnerMutate, isPending: ref(false) }),
    useDeleteParentLink: () => ({ mutateAsync: deleteParentMutate, isPending: ref(false) }),
}))

import { i18n } from '@/i18n'
import PersonDetail from '@/views/tree/PersonDetail.vue'

// Shared stub bundle. v-select renders one `<option>` per item so the test
// can drive selection with `setValue`; v-btn forwards the @click event; the
// expansion-panel stubs render their slot eagerly so we can assert against
// the inner DOM without needing to fake the open/close gesture.
function stubs() {
    return {
        'v-skeleton-loader': { template: '<div class="skeleton" />' },
        'v-list': { template: '<div><slot /></div>' },
        'v-list-item': { template: '<div><slot /></div>' },
        'v-list-item-title': { template: '<div><slot /></div>' },
        'v-list-item-subtitle': { template: '<div><slot /></div>' },
        'v-divider': { template: '<hr />' },
        'v-chip': { template: '<span :data-testid="$attrs[\'data-testid\']"><slot /></span>' },
        'v-expansion-panels': { template: '<div><slot /></div>' },
        'v-expansion-panel': {
            template: '<div :data-testid="$attrs[\'data-testid\']"><slot /></div>',
        },
        'v-expansion-panel-title': { template: '<div><slot /></div>' },
        'v-expansion-panel-text': { template: '<div><slot /></div>' },
        'v-select': {
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
        'v-text-field': {
            template:
                '<input :data-testid="$attrs[\'data-testid\']" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
            props: ['modelValue', 'label', 'type'],
            emits: ['update:modelValue'],
        },
        'v-btn': {
            template: '<button :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\')"><slot /></button>',
            props: ['loading', 'block', 'color', 'variant', 'disabled', 'icon', 'size'],
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
        ContactsSection: {
            // Stub: we just need the testid to exist on the rendered tree so
            // the parent-render assertions can check the section mounts.
            // Dedicated tests cover the ContactsSection internals.
            template: '<section data-testid="contacts-section" />',
            props: ['personId', 'linkedUserId'],
        },
    }
}

function mountDetail(personId: string) {
    return mount(PersonDetail, {
        props: { personId },
        global: {
            plugins: [createPinia(), i18n],
            stubs: stubs(),
        },
    })
}

describe('PersonDetail', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        delMutate.mockReset()
        addParentMutate.mockReset()
        partnerMutate.mockReset()
        updatePartnerMutate.mockReset()
        deletePartnerMutate.mockReset()
        deleteParentMutate.mockReset()
        activeRole.value = 'owner'
        currentUserId.value = 'u-self'
        personGetData.value = undefined
        personGetIsLoading.value = false
        listData.value = [
            {
                id: 'p1',
                given_name: 'A',
                family_name: 'X',
                notes: 'note',
                birth_place: 'Hamburg',
                nickname: 'Ay',
                gender: 'male',
                birth_date: '1965-04-22',
                name_at_birth: '',
                death_date: null,
            },
            {
                id: 'p2',
                given_name: 'B',
                family_name: 'Y',
                birth_place: '',
                nickname: '',
                gender: '',
                name_at_birth: '',
            },
        ]
    })

    it('renders given+family name and notes', () => {
        const w = mountDetail('p1')
        expect(w.text()).toContain('A')
        expect(w.text()).toContain('X')
    })

    it('renders every PersonView field when the GET resolves', () => {
        // Pretend the async GET returned a richer record than the list cache —
        // the drawer renders the GET copy.
        personGetData.value = {
            id: 'p1',
            given_name: 'GivenFromGet',
            family_name: 'FamilyFromGet',
            name_at_birth: 'BirthName',
            nickname: 'NickFromGet',
            gender: 'female',
            birth_date: '1970-01-01',
            birth_place: 'Berlin',
            death_date: '2020-12-31',
            notes: 'Get notes',
        }
        const w = mountDetail('p1')
        expect(w.find('[data-testid="person-field-given-name"]').text()).toContain('GivenFromGet')
        expect(w.find('[data-testid="person-field-name-at-birth"]').text()).toContain('BirthName')
        expect(w.find('[data-testid="person-field-nickname"]').text()).toContain('NickFromGet')
        expect(w.find('[data-testid="person-field-birth-place"]').text()).toContain('Berlin')
        expect(w.find('[data-testid="person-field-death-date"]').text()).toContain('2020-12-31')
        expect(w.find('[data-testid="person-field-notes"]').text()).toContain('Get notes')
        // Phase 3 moved contact data into its own section component — the
        // dedicated ContactsSection tests cover the render path.
        expect(w.find('[data-testid="contacts-section"]').exists()).toBe(true)
    })

    it('owner sees the Edit + Delete actions and no Read-only badge', () => {
        activeRole.value = 'owner'
        const w = mountDetail('p1')
        expect(w.find('[data-testid="person-edit-button"]').exists()).toBe(true)
        expect(w.find('[data-testid="person-delete-button"]').exists()).toBe(true)
        expect(w.find('[data-testid="person-readonly-badge"]').exists()).toBe(false)
        // Add-row inputs (the relations panels) are rendered for editors.
        expect(w.find('[data-testid="person-add-parent"]').exists()).toBe(true)
        expect(w.find('[data-testid="person-add-partner"]').exists()).toBe(true)
    })

    it('user role sees the Read-only badge and no add affordances', () => {
        activeRole.value = 'user'
        const w = mountDetail('p1')
        expect(w.find('[data-testid="person-readonly-badge"]').exists()).toBe(true)
        expect(w.find('[data-testid="person-edit-button"]').exists()).toBe(false)
        expect(w.find('[data-testid="person-delete-button"]').exists()).toBe(false)
        expect(w.find('[data-testid="person-add-parent"]').exists()).toBe(false)
        expect(w.find('[data-testid="person-add-partner"]').exists()).toBe(false)
    })

    it('renders the three relations panels', () => {
        const w = mountDetail('p1')
        expect(w.find('[data-testid="relations-parents"]').exists()).toBe(true)
        expect(w.find('[data-testid="relations-partners"]').exists()).toBe(true)
        expect(w.find('[data-testid="relations-children"]').exists()).toBe(true)
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

    it('"End partnership" pre-fills today + divorce and PATCHes via useUpdatePartnership', async () => {
        updatePartnerMutate.mockResolvedValueOnce(undefined)
        const w = mountDetail('p1')

        // Click the "End partnership" button on the seeded Klaus↔p2 row.
        await w.find('[data-testid="relation-partner-end-part-1"]').trigger('click')

        // The end-date field renders YYYY-MM-DD in local time. We can't
        // hardcode "today" portably, so the assertion checks the format and
        // that the PATCH payload carries an end_reason of 'divorce' plus a
        // non-null ended_on.
        await w.find('[data-testid="relation-partner-save-part-1"]').trigger('click')
        await flushPromises()
        expect(updatePartnerMutate).toHaveBeenCalledTimes(1)
        const callArg = updatePartnerMutate.mock.calls[0]?.[0] as
            | { id: string; input: Record<string, unknown> }
            | undefined
        expect(callArg?.id).toBe('part-1')
        expect(callArg?.input['end_reason']).toBe('divorce')
        expect(typeof callArg?.input['ended_on']).toBe('string')
        expect(callArg?.input['ended_on']).toMatch(/^\d{4}-\d{2}-\d{2}$/)
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

    it('user role can edit their own linked person row', () => {
        activeRole.value = 'user'
        currentUserId.value = 'u-self'
        personGetData.value = {
            id: 'p1',
            given_name: 'A',
            family_name: 'X',
            linked_user_id: 'u-self',
        }
        const w = mountDetail('p1')
        // Edit button is visible; read-only badge is not.
        expect(w.find('[data-testid="person-edit-button"]').exists()).toBe(true)
        expect(w.find('[data-testid="person-readonly-badge"]').exists()).toBe(false)
    })

    it('user role cannot edit someone else’s person row', () => {
        activeRole.value = 'user'
        currentUserId.value = 'u-self'
        personGetData.value = {
            id: 'p1',
            given_name: 'A',
            family_name: 'X',
            linked_user_id: 'u-other',
        }
        const w = mountDetail('p1')
        expect(w.find('[data-testid="person-edit-button"]').exists()).toBe(false)
        expect(w.find('[data-testid="person-readonly-badge"]').exists()).toBe(true)
    })

    it('shows the "Has account" chip when person.linked_user_id is set', () => {
        personGetData.value = {
            id: 'p1',
            given_name: 'A',
            family_name: 'X',
            linked_user_id: 'u-linked',
        }
        const w = mountDetail('p1')
        expect(w.find('[data-testid="person-linked-account-chip"]').exists()).toBe(true)
    })

    it('hides the "Has account" chip when person.linked_user_id is missing', () => {
        personGetData.value = {
            id: 'p1',
            given_name: 'A',
            family_name: 'X',
            linked_user_id: null,
        }
        const w = mountDetail('p1')
        expect(w.find('[data-testid="person-linked-account-chip"]').exists()).toBe(false)
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
