import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import { i18n } from '@/i18n'
import ContactEdit from '@/views/tree/ContactEdit.vue'

const stubs = {
    'v-btn': { template: '<button v-bind="$attrs"><slot /></button>' },
}

// PersonDetail passes the `initial` as a fully-formed contact row, so
// the type requires every field. Tests build full fixtures via this
// helper rather than spread-into-partial.
interface FullInitial {
    id?: string
    kind: string
    label: string
    value: unknown
    visibility: string
}

function mountEdit(initial?: FullInitial) {
    return mount(ContactEdit, {
        props: initial !== undefined ? { initial } : {},
        global: { plugins: [i18n], stubs },
    })
}

describe('ContactEdit', () => {
    it('mounts with empty defaults when no initial contact is provided (kind=email, visibility=family)', () => {
        const w = mountEdit()
        // Native <select>'s `.element.value` reflects the v-model.
        expect((w.find('[data-testid="contact-kind"]').element as HTMLSelectElement).value).toBe('email')
        expect((w.find('[data-testid="contact-visibility"]').element as HTMLSelectElement).value).toBe(
            'family',
        )
        expect((w.find('[data-testid="contact-value"]').element as HTMLInputElement).value).toBe('')
        expect((w.find('[data-testid="contact-label"]').element as HTMLInputElement).value).toBe('')
    })

    it('hydrates from an `email`-kind initial contact, reading the value out of value.email', () => {
        const w = mountEdit({
            id: 'c-1',
            kind: 'email',
            label: 'Work',
            value: { email: 'me@example.com' },
            visibility: 'admins_only',
        })
        expect((w.find('[data-testid="contact-kind"]').element as HTMLSelectElement).value).toBe('email')
        expect((w.find('[data-testid="contact-value"]').element as HTMLInputElement).value).toBe(
            'me@example.com',
        )
        expect((w.find('[data-testid="contact-label"]').element as HTMLInputElement).value).toBe('Work')
        expect((w.find('[data-testid="contact-visibility"]').element as HTMLSelectElement).value).toBe(
            'admins_only',
        )
    })

    it('phone hydrates from `value.number` (BE stores it under the kind-specific key, not `value`)', () => {
        const w = mountEdit({ kind: 'phone', label: '', value: { number: '+49 30 5550100' }, visibility: 'family' })
        expect((w.find('[data-testid="contact-value"]').element as HTMLInputElement).value).toBe(
            '+49 30 5550100',
        )
    })

    it('url + other map to their own kind-specific keys', () => {
        const url = mountEdit({ kind: 'url', label: '', value: { url: 'https://x' }, visibility: 'family' })
        expect((url.find('[data-testid="contact-value"]').element as HTMLInputElement).value).toBe('https://x')

        const other = mountEdit({
            kind: 'other',
            label: '',
            value: { text: 'PGP fingerprint AB CD' },
            visibility: 'family',
        })
        expect((other.find('[data-testid="contact-value"]').element as HTMLInputElement).value).toBe(
            'PGP fingerprint AB CD',
        )
    })

    it('tolerates the legacy `{ v: "..." }` value shape and a bare-string `value`', () => {
        const legacyObj = mountEdit({ kind: 'email', label: '', value: { v: 'legacy@x.com' }, visibility: 'family' })
        expect((legacyObj.find('[data-testid="contact-value"]').element as HTMLInputElement).value).toBe(
            'legacy@x.com',
        )
        const bareString = mountEdit({ kind: 'phone', label: '', value: 'bare-string', visibility: 'family' })
        expect((bareString.find('[data-testid="contact-value"]').element as HTMLInputElement).value).toBe(
            'bare-string',
        )
    })

    it('hydrates from an `address`-kind initial contact, splitting the value into five fields', () => {
        const w = mountEdit({
            kind: 'address',
            label: 'Home',
            value: {
                street: 'Wilhelmstraße',
                house_number: '12a',
                zip: '47877',
                city: 'Willich',
                country: 'DE',
            },
            visibility: 'family',
        })
        // The single-line `contact-value` is replaced by the five address fields.
        expect(w.find('[data-testid="contact-value"]').exists()).toBe(false)
        expect(
            (w.find('[data-testid="contact-address-street"]').element as HTMLInputElement).value,
        ).toBe('Wilhelmstraße')
        expect(
            (w.find('[data-testid="contact-address-house-number"]').element as HTMLInputElement).value,
        ).toBe('12a')
        expect((w.find('[data-testid="contact-address-zip"]').element as HTMLInputElement).value).toBe(
            '47877',
        )
        expect((w.find('[data-testid="contact-address-city"]').element as HTMLInputElement).value).toBe(
            'Willich',
        )
        expect(
            (w.find('[data-testid="contact-address-country"]').element as HTMLInputElement).value,
        ).toBe('DE')
    })

    it('falls back to defaults when initial.kind is an unknown string (sanitise vs hard-fail)', () => {
        const w = mountEdit({
            kind: 'totally-not-a-kind',
            label: '',
            value: { email: 'x@x' },
            visibility: 'who-knows',
        })
        expect((w.find('[data-testid="contact-kind"]').element as HTMLSelectElement).value).toBe('email')
        expect((w.find('[data-testid="contact-visibility"]').element as HTMLSelectElement).value).toBe(
            'family',
        )
    })

    it('switching kind to address hides the single-line value field and shows the address rows', async () => {
        const w = mountEdit()
        const select = w.find('[data-testid="contact-kind"]')
        await select.setValue('address')
        expect(w.find('[data-testid="contact-value"]').exists()).toBe(false)
        expect(w.find('[data-testid="contact-address-street"]').exists()).toBe(true)
    })

    it('save emits the correctly-shaped payload (text kind packs value under the kind key)', async () => {
        const w = mountEdit({ kind: 'email', label: '', value: { email: '' }, visibility: 'family' })
        await w.find('[data-testid="contact-label"]').setValue('Work')
        await w.find('[data-testid="contact-value"]').setValue('a@b.c')
        await w.find('[data-testid="contact-visibility"]').setValue('admins_only')
        await w.find('[data-testid="contact-edit"]').trigger('submit')

        const events = w.emitted('save')
        expect(events).toBeDefined()
        expect(events?.[0]?.[0]).toEqual({
            kind: 'email',
            label: 'Work',
            value: { email: 'a@b.c' },
            visibility: 'admins_only',
        })
    })

    it('save emits the address-shape payload when kind=address', async () => {
        const w = mountEdit({
            kind: 'address',
            label: 'Home',
            value: { street: '', house_number: '', zip: '', city: '', country: '' },
            visibility: 'family',
        })
        await w.find('[data-testid="contact-address-street"]').setValue('Marktplatz')
        await w.find('[data-testid="contact-address-house-number"]').setValue('1')
        await w.find('[data-testid="contact-address-zip"]').setValue('10115')
        await w.find('[data-testid="contact-address-city"]').setValue('Berlin')
        await w.find('[data-testid="contact-address-country"]').setValue('DE')
        await w.find('[data-testid="contact-edit"]').trigger('submit')

        expect(w.emitted('save')?.[0]?.[0]).toEqual({
            kind: 'address',
            label: 'Home',
            value: {
                street: 'Marktplatz',
                house_number: '1',
                zip: '10115',
                city: 'Berlin',
                country: 'DE',
            },
            visibility: 'family',
        })
    })

    it('cancel emits `cancel` and does NOT emit save', async () => {
        const w = mountEdit()
        await w.find('[data-testid="contact-cancel"]').trigger('click')
        expect(w.emitted('cancel')).toBeDefined()
        expect(w.emitted('save')).toBeUndefined()
    })
})
