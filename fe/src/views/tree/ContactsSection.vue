<script setup lang="ts">
import { computed, ref, toRef } from 'vue'
import { useI18n } from 'vue-i18n'

import {
    useContacts,
    useCreateContact,
    useDeleteContact,
    useUpdateContact,
    type ContactInput,
} from '@/api/hooks/contacts'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

import ContactEdit from './ContactEdit.vue'

const props = defineProps<{
    /** The person whose contacts we're rendering. */
    personId: string
    /**
     * Linked user id on the person row — the FE's local mirror of the
     * backend's edit gate. The server still enforces it; this flag
     * just hides the Add/Edit/Delete affordances for `user`-role
     * members on rows they can't edit.
     */
    linkedUserId: string | null
}>()

const { t } = useI18n()
const family = useActiveFamilyStore()
const auth = useAuthStore()

// `toRef(props, 'personId')` keeps useContacts' query key in sync with the
// drawer navigation — when the parent swaps `:person-id` we refetch
// against the new id instead of staying stale on the first-mount value.
const personIdRef = toRef(props, 'personId')
const list = useContacts(personIdRef)
const create = useCreateContact(props.personId)
const update = useUpdateContact(props.personId)
const remove = useDeleteContact(props.personId)

const editingId = ref<string | null>(null)
const creating = ref(false)

/**
 * Mirrors the backend rule: admin/owner may edit anyone's contacts;
 * `user` role may only edit contacts on their own linked person.
 * `user_id` lives on the auth store but we ask the active family for
 * the current role since the JWT membership is the source of truth.
 */
const canEdit = computed(() => {
    const role = family.activeFamily?.role ?? null
    if (role === 'owner' || role === 'admin') return true
    if (role === 'user') {
        const myUserId = auth.user?.id ?? null
        return props.linkedUserId !== null && myUserId !== null && props.linkedUserId === myUserId
    }
    return false
})

function startEdit(id: string): void {
    creating.value = false
    editingId.value = id
}

function startCreate(): void {
    editingId.value = null
    creating.value = true
}

async function onSave(value: ContactInput): Promise<void> {
    if (creating.value) {
        await create.mutateAsync(value)
        creating.value = false
        return
    }
    const id = editingId.value
    if (id === null) return
    await update.mutateAsync({ id, input: value })
    editingId.value = null
}

function onCancel(): void {
    editingId.value = null
    creating.value = false
}

function displayValue(kind: string, v: unknown): string {
    if (kind === 'address') {
        if (typeof v !== 'object' || v === null) return ''
        const o = v as Record<string, unknown>
        const s = (k: string): string => (typeof o[k] === 'string' ? (o[k] as string) : '')
        const line1 = [s('street'), s('house_number')].filter((p) => p !== '').join(' ')
        const line2 = [s('zip'), s('city')].filter((p) => p !== '').join(' ')
        const country = s('country')
        return [line1, line2, country].filter((p) => p !== '').join(', ')
    }
    if (typeof v === 'string') return v
    if (typeof v === 'object' && v !== null) {
        const obj = v as Record<string, unknown>
        const key = kind === 'email' ? 'email' : kind === 'phone' ? 'number' : kind === 'url' ? 'url' : 'text'
        if (typeof obj[key] === 'string') return obj[key] as string
        if (typeof obj['v'] === 'string') return obj['v'] as string
    }
    return ''
}

function editingInitial(id: string): {
    id: string
    kind: string
    label: string
    value: unknown
    visibility: string
} | null {
    const found = list.data.value?.find((c) => c.id === id)
    if (found === undefined) return null
    return {
        id: found.id,
        kind: found.kind,
        label: found.label,
        value: found.value,
        visibility: found.visibility,
    }
}
</script>

<template>
    <section class="contacts" data-testid="contacts-section">
        <div class="d-flex align-center justify-space-between mb-2">
            <h4 class="text-subtitle-1">{{ t('contact.heading') }}</h4>
            <v-btn
                v-if="canEdit && !creating && editingId === null"
                size="small"
                variant="outlined"
                data-testid="contact-add"
                @click="startCreate"
            >
                {{ t('contact.add') }}
            </v-btn>
        </div>

        <p v-if="list.isLoading.value" class="text-body-2">{{ t('common.loading') }}</p>
        <ul v-else-if="list.data.value && list.data.value.length > 0" class="contact-list">
            <li v-for="c in list.data.value" :key="c.id" class="contact-row" :data-testid="`contact-row-${c.id}`">
                <div class="contact-summary">
                    <!-- Shared `ds-form-*` primitives from
                         design-system/forms.css so this row and the
                         contact-edit form share a single source of
                         truth for theme-aware label / value / chip
                         colours. -->
                    <span class="ds-form-label">{{ t(`contact.kinds.${c.kind}`) }}</span>
                    <span v-if="c.label !== ''" class="ds-form-sublabel">{{ c.label }}</span>
                    <span class="ds-form-value">{{ displayValue(c.kind, c.value) }}</span>
                    <span v-if="c.visibility === 'admins_only'" class="ds-form-chip" :title="t('contact.visAdmins')">
                        {{ t('contact.visAdmins') }}
                    </span>
                </div>
                <div v-if="canEdit" class="contact-actions">
                    <v-btn size="x-small" variant="text" :data-testid="`contact-edit-${c.id}`" @click="startEdit(c.id)">
                        {{ t('common.edit') }}
                    </v-btn>
                    <v-btn
                        size="x-small"
                        variant="text"
                        color="error"
                        :data-testid="`contact-delete-${c.id}`"
                        @click="remove.mutateAsync(c.id)"
                    >
                        {{ t('common.delete') }}
                    </v-btn>
                </div>
            </li>
        </ul>
        <p v-else class="text-body-2 text-medium-emphasis">{{ t('contact.empty') }}</p>

        <ContactEdit
            v-if="editingId !== null && editingInitial(editingId) !== null"
            :key="`edit-${editingId}`"
            :initial="editingInitial(editingId)!"
            @save="onSave"
            @cancel="onCancel"
        />
        <ContactEdit v-if="creating" key="create" @save="onSave" @cancel="onCancel" />
    </section>
</template>

<style scoped>
/* Layout-only styles here — the per-row label/value/chip colouring
 * lives in the shared `design-system/forms.css` primitives so this
 * read-only display and the edit form (`ContactEdit.vue`) can't
 * drift apart. */
.contacts {
    margin-top: 0.5rem;
}
.contact-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.4rem;
}
.contact-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.35rem 0;
    border-bottom: 1px solid var(--border);
}
.contact-summary {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    flex-wrap: wrap;
}
.contact-actions {
    display: flex;
    gap: 0.25rem;
}
</style>
