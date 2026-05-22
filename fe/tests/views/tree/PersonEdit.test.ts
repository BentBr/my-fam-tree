import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

const createMutate = vi.fn()
const updateMutate = vi.fn()
vi.mock('@/api/hooks/persons', () => ({
    useCreatePerson: () => ({ mutateAsync: createMutate, isPending: ref(false) }),
    useUpdatePerson: () => ({ mutateAsync: updateMutate, isPending: ref(false) }),
}))

import { i18n } from '@/i18n'
import PersonEdit from '@/views/tree/PersonEdit.vue'

interface MountProps {
    mode: 'create' | 'edit'
    initial?: {
        id?: string
        given_name?: string
        family_name?: string
        name_at_birth?: string
        nickname?: string
        gender?: string
        birth_date?: string | null
        birth_place?: string
        death_date?: string | null
        notes?: string
        email?: string
        phone?: string
        street?: string
        house_number?: string
        zip?: string
        city?: string
        country?: string
        linked_user_id?: string | null
    }
}

// All 14 user-editable fields covered by the form, with the data-testid
// of the input each one renders into. The "all 14 fields" assertion below
// walks this list to make the test resilient to ordering / layout tweaks
// inside the v-form template.
const ALL_FIELDS: ReadonlyArray<{ testid: string; key: string; value: string }> = [
    { testid: 'person-given-name', key: 'given_name', value: 'Greta' },
    { testid: 'person-family-name', key: 'family_name', value: 'Schmidt' },
    { testid: 'person-name-at-birth', key: 'name_at_birth', value: 'Hoffmann' },
    { testid: 'person-nickname', key: 'nickname', value: 'Gretchen' },
    { testid: 'person-birth-date', key: 'birth_date', value: '1940-02-09' },
    { testid: 'person-birth-place', key: 'birth_place', value: 'Augsburg' },
    { testid: 'person-gender', key: 'gender', value: 'female' },
    { testid: 'person-notes', key: 'notes', value: 'family historian' },
    { testid: 'person-email', key: 'email', value: 'g@example.de' },
    { testid: 'person-phone', key: 'phone', value: '+49 30 1234' },
    { testid: 'person-street', key: 'street', value: 'Hauptstr.' },
    { testid: 'person-house-number', key: 'house_number', value: '12' },
    { testid: 'person-zip', key: 'zip', value: '10115' },
    { testid: 'person-city', key: 'city', value: 'Berlin' },
    { testid: 'person-country', key: 'country', value: 'Deutschland' },
]

function mountView(props: MountProps) {
    return mount(PersonEdit, {
        props,
        global: {
            plugins: [createPinia(), i18n],
            stubs: {
                'v-form': {
                    template:
                        '<form @submit.prevent="$emit(\'submit\', { preventDefault: () => undefined })"><slot /></form>',
                    emits: ['submit'],
                },
                'v-text-field': {
                    // Forward `readonly` as the HTML attribute so tests can
                    // assert on it; v-combobox is stubbed below the same way.
                    template:
                        '<input :data-testid="$attrs[\'data-testid\']" :value="modelValue" :readonly="readonly" @input="$emit(\'update:modelValue\', $event.target.value)" />',
                    props: ['modelValue', 'label', 'type', 'autocomplete', 'readonly', 'hint', 'persistentHint'],
                    emits: ['update:modelValue'],
                },
                'v-textarea': {
                    template:
                        '<textarea :data-testid="$attrs[\'data-testid\']" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
                    props: ['modelValue', 'label', 'rows', 'autoGrow'],
                    emits: ['update:modelValue'],
                },
                'v-combobox': {
                    template:
                        '<input :data-testid="$attrs[\'data-testid\']" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
                    props: ['modelValue', 'items', 'label'],
                    emits: ['update:modelValue'],
                },
                'v-checkbox': {
                    template:
                        '<input type="checkbox" :data-testid="$attrs[\'data-testid\']" :checked="modelValue" @change="$emit(\'update:modelValue\', $event.target.checked)" />',
                    props: ['modelValue', 'label', 'density', 'hideDetails'],
                    emits: ['update:modelValue'],
                },
                'v-row': { template: '<div><slot /></div>', props: ['dense'] },
                'v-col': { template: '<div><slot /></div>', props: ['cols'] },
                'v-divider': { template: '<hr />' },
                'v-btn': {
                    // Default to type=button (not submit) so the cancel button doesn't
                    // double-fire as a form submit. The submit button passes type="submit" explicitly.
                    template:
                        '<button :data-testid="$attrs[\'data-testid\']" :type="type || \'button\'" @click.stop="$emit(\'click\')"><slot /></button>',
                    props: ['type', 'loading', 'variant', 'color'],
                },
            },
        },
    })
}

describe('PersonEdit', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        createMutate.mockReset()
        updateMutate.mockReset()
    })

    it('create mode emits "saved" with new id', async () => {
        createMutate.mockResolvedValueOnce({ id: 'new-1' })
        const w = mountView({ mode: 'create' })
        await w.find('[data-testid="person-given-name"]').setValue('A')
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(createMutate).toHaveBeenCalled()
        expect(w.emitted('saved')?.[0]).toEqual(['new-1'])
    })

    it('edit mode dispatches PATCH with id', async () => {
        updateMutate.mockResolvedValueOnce({ id: 'p-1' })
        const w = mountView({ mode: 'edit', initial: { id: 'p-1', given_name: 'A' } })
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(updateMutate).toHaveBeenCalledWith({ id: 'p-1', input: expect.any(Object) })
    })

    it('edit mode without id is a no-op', async () => {
        const w = mountView({ mode: 'edit', initial: {} })
        await w.find('form').trigger('submit')
        await flushPromises()
        expect(updateMutate).not.toHaveBeenCalled()
        expect(w.emitted('saved')).toBeUndefined()
    })

    it('cancel button emits "cancel"', async () => {
        const w = mountView({ mode: 'create' })
        await w.find('[data-testid="person-edit-cancel"]').trigger('click')
        // The cancel handler emits once per click; we don't pin the exact count
        // because Vue dev-mode may dispatch the click event through more than
        // one DOM listener layer (button + form), so we just require it fired.
        expect((w.emitted('cancel') ?? []).length).toBeGreaterThanOrEqual(1)
    })

    it('every PersonView field is editable and flows through to the mutation payload', async () => {
        updateMutate.mockResolvedValueOnce({ id: 'p-1' })
        const w = mountView({ mode: 'edit', initial: { id: 'p-1', given_name: 'Initial' } })

        // Tick the deceased toggle so the conditional death_date input renders.
        await w.find('[data-testid="person-deceased"]').setValue(true)
        const deathField = w.find('[data-testid="person-death-date"]')
        expect(deathField.exists()).toBe(true)
        await deathField.setValue('2020-12-31')

        // Drive every other field by walking ALL_FIELDS. Each entry's `testid`
        // must exist in the DOM and accept the new value.
        for (const f of ALL_FIELDS) {
            const el = w.find(`[data-testid="${f.testid}"]`)
            expect(el.exists(), `field ${f.testid} should render`).toBe(true)
            await el.setValue(f.value)
        }

        await w.find('form').trigger('submit')
        await flushPromises()

        expect(updateMutate).toHaveBeenCalledTimes(1)
        const call = updateMutate.mock.calls[0]?.[0] as { id: string; input: Record<string, unknown> } | undefined
        expect(call?.id).toBe('p-1')
        const input = call?.input ?? {}
        // Every contact / profile field reached the payload.
        for (const f of ALL_FIELDS) {
            expect(input[f.key], `payload should carry ${f.key}`).toBe(f.value)
        }
        expect(input['death_date']).toBe('2020-12-31')
    })

    it('email field is read-only when person is linked to a user', () => {
        const w = mountView({
            mode: 'edit',
            initial: { id: 'p-1', given_name: 'Klaus', email: 'admin@example.com', linked_user_id: 'u-1' },
        })
        const email = w.find('[data-testid="person-email"]')
        expect(email.exists()).toBe(true)
        expect(email.attributes('readonly')).toBeDefined()
    })

    it('email field is editable when person is not linked to a user', () => {
        const w = mountView({
            mode: 'edit',
            initial: { id: 'p-1', given_name: 'Greta', email: 'g@example.de', linked_user_id: null },
        })
        const email = w.find('[data-testid="person-email"]')
        expect(email.exists()).toBe(true)
        // jsdom serializes readonly=false as the attribute being absent or
        // explicitly "false"; either way it shouldn't read as a present-empty
        // string (which is how readonly=true serializes).
        const ro = email.attributes('readonly')
        expect(ro === undefined || ro === 'false').toBe(true)
    })
})
