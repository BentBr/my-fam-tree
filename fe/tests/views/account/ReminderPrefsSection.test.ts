import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

import type { ReminderPrefs } from '@/api/hooks/reminders'

const saveMutate = vi.fn()
const prefsData = ref<ReminderPrefs | undefined>(undefined)

vi.mock('@/api/hooks/reminders', () => ({
    useReminderPrefs: () => ({ data: prefsData }),
    useSaveReminderPrefs: () => ({ mutateAsync: saveMutate, isPending: ref(false) }),
}))

import { i18n } from '@/i18n'
import ReminderPrefsSection from '@/views/account/ReminderPrefsSection.vue'

const stubs = {
    'v-card-subtitle': { template: '<div><slot /></div>' },
    'v-switch': {
        template:
            '<input type="checkbox" :data-testid="$attrs[\'data-testid\']" :disabled="disabled" :checked="modelValue" @change="$emit(\'update:modelValue\', $event.target.checked)" />',
        props: ['modelValue', 'disabled', 'label', 'color', 'density', 'hideDetails'],
        emits: ['update:modelValue'],
    },
    'v-slider': {
        template:
            '<input type="range" :data-testid="$attrs[\'data-testid\']" :disabled="disabled" :value="modelValue" @input="$emit(\'update:modelValue\', Number($event.target.value))" />',
        props: ['modelValue', 'disabled', 'min', 'max', 'step', 'thumbLabel', 'label'],
        emits: ['update:modelValue'],
    },
    'v-btn': {
        template: '<button :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\')"><slot /></button>',
        props: ['loading', 'block', 'color'],
    },
}

function mountSection(): ReturnType<typeof mount> {
    return mount(ReminderPrefsSection, { global: { plugins: [i18n], stubs } })
}

describe('ReminderPrefsSection', () => {
    beforeEach(() => {
        prefsData.value = undefined
        saveMutate.mockReset()
    })

    it('gates the child controls behind the master switch when emails are off', async () => {
        const w = mountSection()
        prefsData.value = {
            emails_enabled: false,
            remind_birthdays: true,
            remind_anniversaries: false,
            favourites_only: true,
            lead_days: 3,
        }
        await flushPromises()
        expect(w.find('[data-testid="reminder-birthdays"]').attributes('disabled')).toBeDefined()
        expect(w.find('[data-testid="reminder-anniversaries"]').attributes('disabled')).toBeDefined()
        expect(w.find('[data-testid="reminder-favourites-only"]').attributes('disabled')).toBeDefined()
        expect(w.find('[data-testid="reminder-lead-days"]').attributes('disabled')).toBeDefined()
    })

    it('enables the child controls when the master switch is on', async () => {
        const w = mountSection()
        prefsData.value = {
            emails_enabled: true,
            remind_birthdays: true,
            remind_anniversaries: true,
            favourites_only: false,
            lead_days: 7,
        }
        await flushPromises()
        expect(w.find('[data-testid="reminder-birthdays"]').attributes('disabled')).toBeUndefined()
        expect(w.find('[data-testid="reminder-lead-days"]').attributes('disabled')).toBeUndefined()
    })

    it('saves the current form values on click', async () => {
        const w = mountSection()
        prefsData.value = {
            emails_enabled: true,
            remind_birthdays: true,
            remind_anniversaries: false,
            favourites_only: true,
            lead_days: 14,
        }
        await flushPromises()
        saveMutate.mockResolvedValueOnce(undefined)
        await w.find('[data-testid="reminder-save"]').trigger('click')
        await flushPromises()
        expect(saveMutate).toHaveBeenCalledWith({
            emails_enabled: true,
            remind_birthdays: true,
            remind_anniversaries: false,
            favourites_only: true,
            lead_days: 14,
        })
    })

    it('toggling the master switch flips the form value', async () => {
        const w = mountSection()
        prefsData.value = {
            emails_enabled: false,
            remind_birthdays: true,
            remind_anniversaries: true,
            favourites_only: false,
            lead_days: 7,
        }
        await flushPromises()
        await w.find('[data-testid="reminder-emails-enabled"]').setValue(true)
        saveMutate.mockResolvedValueOnce(undefined)
        await w.find('[data-testid="reminder-save"]').trigger('click')
        await flushPromises()
        expect(saveMutate).toHaveBeenCalledWith(expect.objectContaining({ emails_enabled: true }))
    })
})
