<script setup lang="ts">
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'

import type { ContactInput, ContactKind, ContactVisibility } from '@/api/hooks/contacts'

interface AddressShape {
    street: string
    house_number: string
    zip: string
    city: string
    country: string
}

interface InitialContact {
    id?: string
    kind: string
    label: string
    value: unknown
    visibility: string
}

const props = defineProps<{ initial?: InitialContact }>()
const emit = defineEmits<{
    (e: 'save', value: ContactInput): void
    (e: 'cancel'): void
}>()

const { t } = useI18n()

interface FormShape {
    kind: ContactKind
    label: string
    /** Flat-text value for the single-line kinds (email/phone/url/other). */
    textValue: string
    /** Structured address payload for `kind == 'address'`. */
    address: AddressShape
    visibility: ContactVisibility
}

const KINDS = ['email', 'phone', 'address', 'url', 'other'] as const
const VISIBILITIES = ['family', 'admins_only'] as const

function isKind(v: string): v is ContactKind {
    return (KINDS as readonly string[]).includes(v)
}
function isVisibility(v: string): v is ContactVisibility {
    return (VISIBILITIES as readonly string[]).includes(v)
}

/**
 * Pull the text-value out of a contact `value` for the single-line kinds
 * (`email`/`phone`/`url`/`other`). The backend stores each kind under a
 * self-documenting key (`email`/`number`/`url`/`text`); we also accept the
 * legacy `{ v: "..." }` shape and bare strings for older rows.
 */
function extractTextValue(initial: InitialContact | undefined): string {
    if (initial === undefined) return ''
    const v = initial.value
    if (typeof v === 'string') return v
    if (typeof v !== 'object' || v === null) return ''
    const obj = v as Record<string, unknown>
    const key = textValueKey(initial.kind)
    if (typeof obj[key] === 'string') return obj[key] as string
    if (typeof obj['v'] === 'string') return obj['v'] as string
    return ''
}

/**
 * The JSONB key the backend uses for the text-value of a given kind.
 * Mirrors `crates/api/src/routes/contacts.rs` — each kind has a
 * self-documenting field name; the FE writes the same shape on POST/PATCH.
 */
function textValueKey(kind: string): string {
    switch (kind) {
        case 'email':
            return 'email'
        case 'phone':
            return 'number'
        case 'url':
            return 'url'
        case 'other':
        default:
            return 'text'
    }
}

function extractAddress(initial: InitialContact | undefined): AddressShape {
    const empty: AddressShape = { street: '', house_number: '', zip: '', city: '', country: '' }
    if (initial === undefined) return empty
    if (initial.kind !== 'address') return empty
    const v = initial.value
    if (typeof v !== 'object' || v === null) return empty
    const obj = v as Record<string, unknown>
    const s = (key: string): string => (typeof obj[key] === 'string' ? (obj[key] as string) : '')
    return {
        street: s('street'),
        house_number: s('house_number'),
        zip: s('zip'),
        city: s('city'),
        country: s('country'),
    }
}

const rawKind = props.initial?.kind ?? ''
const rawVis = props.initial?.visibility ?? ''
const initialKind: ContactKind = isKind(rawKind) ? rawKind : 'email'
const initialVis: ContactVisibility = isVisibility(rawVis) ? rawVis : 'family'

const form = reactive<FormShape>({
    kind: initialKind,
    label: props.initial?.label ?? '',
    textValue: extractTextValue(props.initial),
    address: extractAddress(props.initial),
    visibility: initialVis,
})

function buildValue(): unknown {
    if (form.kind === 'address') {
        return {
            street: form.address.street,
            house_number: form.address.house_number,
            zip: form.address.zip,
            city: form.address.city,
            country: form.address.country,
        }
    }
    return { [textValueKey(form.kind)]: form.textValue }
}

function submit(): void {
    emit('save', {
        kind: form.kind,
        label: form.label,
        value: buildValue(),
        visibility: form.visibility,
    })
}
</script>

<template>
    <!-- All label / input / row styling comes from the shared
         `ds-form-*` primitives in `design-system/forms.css` so this
         form stays readable in both light + dark themes without
         duplicating the per-component rgba(0,0,0,…) constants that
         used to fade to invisible on dark surfaces. ContactsSection
         (the read-only display) uses the same primitives. -->
    <form class="contact-edit" data-testid="contact-edit" @submit.prevent="submit">
        <label class="ds-form-row">
            <span class="ds-form-label">{{ t('contact.kind') }}</span>
            <select v-model="form.kind" class="ds-form-input" data-testid="contact-kind">
                <option v-for="k in KINDS" :key="k" :value="k">{{ t(`contact.kinds.${k}`) }}</option>
            </select>
        </label>
        <label class="ds-form-row">
            <span class="ds-form-label">{{ t('contact.label') }}</span>
            <input v-model="form.label" class="ds-form-input" type="text" data-testid="contact-label" />
        </label>
        <template v-if="form.kind === 'address'">
            <label class="ds-form-row">
                <span class="ds-form-label">{{ t('contact.address.street') }}</span>
                <input
                    v-model="form.address.street"
                    class="ds-form-input"
                    type="text"
                    data-testid="contact-address-street"
                />
            </label>
            <label class="ds-form-row">
                <span class="ds-form-label">{{ t('contact.address.house_number') }}</span>
                <input
                    v-model="form.address.house_number"
                    class="ds-form-input"
                    type="text"
                    data-testid="contact-address-house-number"
                />
            </label>
            <label class="ds-form-row">
                <span class="ds-form-label">{{ t('contact.address.zip') }}</span>
                <input
                    v-model="form.address.zip"
                    class="ds-form-input"
                    type="text"
                    data-testid="contact-address-zip"
                />
            </label>
            <label class="ds-form-row">
                <span class="ds-form-label">{{ t('contact.address.city') }}</span>
                <input
                    v-model="form.address.city"
                    class="ds-form-input"
                    type="text"
                    data-testid="contact-address-city"
                />
            </label>
            <label class="ds-form-row">
                <span class="ds-form-label">{{ t('contact.address.country') }}</span>
                <input
                    v-model="form.address.country"
                    class="ds-form-input"
                    type="text"
                    data-testid="contact-address-country"
                />
            </label>
        </template>
        <label v-else class="ds-form-row">
            <span class="ds-form-label">{{ t('contact.value') }}</span>
            <input
                v-model="form.textValue"
                class="ds-form-input"
                type="text"
                required
                data-testid="contact-value"
            />
        </label>
        <label class="ds-form-row">
            <span class="ds-form-label">{{ t('contact.visibility') }}</span>
            <select v-model="form.visibility" class="ds-form-input" data-testid="contact-visibility">
                <option value="family">{{ t('contact.visFamily') }}</option>
                <option value="admins_only">{{ t('contact.visAdmins') }}</option>
            </select>
        </label>
        <div class="actions">
            <v-btn variant="text" data-testid="contact-cancel" @click="emit('cancel')">
                {{ t('common.cancel') }}
            </v-btn>
            <v-btn type="submit" color="primary" data-testid="contact-submit">
                {{ t('common.save') }}
            </v-btn>
        </div>
    </form>
</template>

<style scoped>
.contact-edit {
    display: grid;
    gap: 0.5rem;
    padding: 0.5rem 0;
}
.actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.5rem;
}
</style>
