import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

// Output-escaping regression guard (Phase 5 security review, task 21).
//
// Contact `label` and `value` are user-controlled free text. The component
// renders them with `{{ }}` interpolation, which Vue HTML-escapes — there is
// NO `v-html`/`innerHTML` sink anywhere in the contact/person render path, and
// the `url` kind is rendered as plain text (never an `href`), so neither
// stored-XSS nor `javascript:` scheme injection is reachable. This test pins
// that behaviour: a malicious payload must surface as inert text, never as a
// live DOM element.

interface Contact {
    id: string
    kind: string
    label: string
    value: unknown
    visibility: string
}

const contactsData = ref<Contact[] | undefined>([])

vi.mock('@/stores/activeFamily', () => ({
    useActiveFamilyStore: () => ({ activeFamily: { id: 'fam', name: 'Müller', role: 'owner' } }),
}))

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({ user: { id: 'u-self', email: 'a@b.c', locale: 'en', displayName: '' } }),
}))

vi.mock('@/api/hooks/contacts', () => ({
    useContacts: () => ({ data: contactsData, isLoading: ref(false), error: ref(null) }),
    useCreateContact: () => ({ mutateAsync: vi.fn(), isPending: ref(false) }),
    useUpdateContact: () => ({ mutateAsync: vi.fn(), isPending: ref(false) }),
    useDeleteContact: () => ({ mutateAsync: vi.fn(), isPending: ref(false) }),
}))

import { i18n } from '@/i18n'
import ContactsSection from '@/views/tree/ContactsSection.vue'

function stubs() {
    return {
        'v-btn': {
            template: '<button :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\')"><slot /></button>',
            props: ['size', 'variant', 'color'],
            emits: ['click'],
        },
        ContactEdit: { template: '<div class="edit-stub" />', props: ['initial'], emits: ['save', 'cancel'] },
    }
}

function mountSection() {
    return mount(ContactsSection, {
        props: { personId: 'p1', linkedUserId: null },
        global: { plugins: [createPinia(), i18n], stubs: stubs() },
    })
}

const IMG_PAYLOAD = '<img src=x onerror=alert(1)>'
const SCRIPT_PAYLOAD = '<script>alert(1)</' + 'script>'

describe('ContactsSection output escaping', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        contactsData.value = []
    })

    it('renders a malicious contact value/label as escaped text, not live DOM', () => {
        contactsData.value = [
            {
                id: 'c1',
                kind: 'other',
                label: IMG_PAYLOAD,
                value: SCRIPT_PAYLOAD,
                visibility: 'family',
            },
        ]
        const w = mountSection()
        const row = w.find('[data-testid="contact-row-c1"]')
        expect(row.exists()).toBe(true)

        // The payload is visible to the user as literal text…
        expect(row.text()).toContain(IMG_PAYLOAD)
        expect(row.text()).toContain(SCRIPT_PAYLOAD)

        // …but Vue escaped it: no live <img>/<script> element was injected,
        // and the serialised HTML carries the entity-encoded `&lt;`.
        expect(row.element.querySelector('img')).toBeNull()
        expect(row.element.querySelector('script')).toBeNull()
        expect(row.html()).toContain('&lt;img')
        expect(row.html()).not.toContain('<img')
    })

    it('renders a url-kind contact as inert text (no href / javascript: sink)', () => {
        contactsData.value = [
            {
                id: 'c2',
                kind: 'url',
                label: 'site',
                value: { url: 'javascript:alert(document.cookie)' },
                visibility: 'family',
            },
        ]
        const w = mountSection()
        const row = w.find('[data-testid="contact-row-c2"]')
        expect(row.exists()).toBe(true)
        // The URL is shown as plain text — it never becomes a clickable link,
        // so a `javascript:`/`data:` scheme can't be navigated to.
        expect(row.text()).toContain('javascript:alert(document.cookie)')
        expect(row.element.querySelector('a')).toBeNull()
    })
})
