<script setup lang="ts">
import { computed, ref, toRef } from 'vue'
import { useI18n } from 'vue-i18n'

import { useCreateInvite } from '@/api/hooks/invites'
import { useDeletePerson, useGetPerson, useListPersons } from '@/api/hooks/persons'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

import ContactsSection from './ContactsSection.vue'
import PersonEdit from './PersonEdit.vue'
import PersonRelations from './PersonRelations.vue'

const props = defineProps<{
    personId: string
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'changed'): void
}>()

const { t } = useI18n()

// `toRef(props, 'personId')` lets the useGetPerson query react to drawer
// navigation without remounting the component.
const personId = toRef(props, 'personId')

// Two data sources:
// - `useGetPerson` is the source of truth for the rendered profile fields —
//   per the brief, the drawer must always reflect the latest server state.
// - `useListPersons` is the fallback (also used by tests + while the GET is
//   in flight) so the header never flashes a blank name.
const personQ = useGetPerson(personId)
const list = useListPersons()
const del = useDeletePerson()

const family = useActiveFamilyStore()
const auth = useAuthStore()

const editing = ref(false)
const confirmDelete = ref(false)

const person = computed(() => personQ.data.value ?? list.data.value?.find((p) => p.id === props.personId) ?? null)

// owner+admin can edit any row. A `user`-role member can only edit the
// person row that maps to their own user (linked_user_id matches the
// signed-in user). `null` role happens during the brief window after
// sign-in before the active-family hydrates — render read-only then.
const canEdit = computed(() => {
    const role = family.activeFamily?.role ?? null
    if (role === 'owner' || role === 'admin') return true
    if (role === 'user' && person.value?.linked_user_id !== null && person.value?.linked_user_id !== undefined) {
        return person.value.linked_user_id === auth.user?.id
    }
    return false
})

// Invite-to-family modal. Visible to admin+owner only; gated on the
// active family's role (read from the same store as `canEdit` above so
// the two checks share their loading-window semantics).
const inviteMutation = useCreateInvite()
const inviteOpen = ref(false)
const inviteEmail = ref('')
const inviteRole = ref<'user' | 'admin'>('user')

const canInvite = computed(() => {
    const role = family.activeFamily?.role ?? null
    return role === 'admin' || role === 'owner'
})

function openInvite(): void {
    inviteEmail.value = ''
    inviteRole.value = 'user'
    inviteOpen.value = true
}

async function submitInvite(): Promise<void> {
    if (person.value === null) return
    await inviteMutation.mutateAsync({
        email: inviteEmail.value.trim(),
        role: inviteRole.value,
        personId: person.value.id,
    })
    inviteOpen.value = false
}

async function remove(): Promise<void> {
    await del.mutateAsync(props.personId)
    confirmDelete.value = false
    emit('changed')
    emit('close')
}

function onSaved(): void {
    editing.value = false
    emit('changed')
}

function onRelationsChanged(): void {
    emit('changed')
}
</script>

<template>
    <section class="pa-4" data-testid="person-detail">
        <header class="d-flex align-center justify-space-between mb-3">
            <div class="d-flex align-center ga-2">
                <h3 v-if="person" class="text-h6" data-testid="person-detail-title">
                    {{ person.given_name }} {{ person.family_name }}
                </h3>
                <v-chip
                    v-if="person?.linked_user_id"
                    size="small"
                    color="info"
                    variant="tonal"
                    prepend-icon="user-check"
                    data-testid="person-linked-account-chip"
                >
                    {{ t('person.linkedAccount') }}
                </v-chip>
            </div>
            <v-btn icon="x" variant="text" size="small" data-testid="person-detail-close" @click="emit('close')" />
        </header>

        <v-skeleton-loader v-if="personQ.isLoading.value && person === null" type="article" />

        <template v-else-if="!editing && person !== null">
            <div class="d-flex align-center justify-space-between mb-2">
                <h4 class="text-subtitle-1">{{ t('person.sections.profile') }}</h4>
                <v-chip
                    v-if="!canEdit"
                    size="small"
                    color="grey-lighten-1"
                    variant="tonal"
                    data-testid="person-readonly-badge"
                >
                    {{ t('common.readOnly') }}
                </v-chip>
            </div>

            <!-- Profile section — every PersonView field rendered as a labelled
                 read-only row. Editing flips into the existing PersonEdit
                 component (covers every field via v-text-field/v-combobox);
                 we deliberately don't duplicate that form here. -->
            <v-list density="compact" class="mb-2" data-testid="person-profile-list">
                <v-list-item data-testid="person-field-given-name">
                    <v-list-item-title>{{ t('person.fields.given_name') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.given_name }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-family-name">
                    <v-list-item-title>{{ t('person.fields.family_name') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.family_name || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-name-at-birth">
                    <v-list-item-title>{{ t('person.fields.name_at_birth') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.name_at_birth || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-nickname">
                    <v-list-item-title>{{ t('person.fields.nickname') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.nickname || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-gender">
                    <v-list-item-title>{{ t('person.fields.gender') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.gender || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-birth-date">
                    <v-list-item-title>{{ t('person.fields.birth_date') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.birth_date ?? '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-birth-place">
                    <v-list-item-title>{{ t('person.fields.birth_place') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.birth_place || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-death-date">
                    <v-list-item-title>{{ t('person.fields.death_date') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.death_date ?? '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item v-if="person.notes" data-testid="person-field-notes">
                    <v-list-item-title>{{ t('person.fields.notes') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.notes }}</v-list-item-subtitle>
                </v-list-item>
            </v-list>

            <v-divider class="my-3" />

            <!-- Contacts section. Replaces the flat email/phone/address
                 fields with per-row entries from `person_contacts`. The
                 backend filters `admins_only` rows out for `user` role. -->
            <ContactsSection :person-id="props.personId" :linked-user-id="person.linked_user_id ?? null" />

            <v-divider class="my-3" />

            <PersonRelations :person-id="props.personId" :can-edit="canEdit" @changed="onRelationsChanged" />

            <v-divider class="my-3" />

            <div v-if="canEdit" class="d-flex ga-2 flex-wrap">
                <v-btn variant="outlined" data-testid="person-edit-button" @click="editing = true">
                    {{ t('common.edit') }}
                </v-btn>
                <v-btn
                    v-if="canInvite"
                    variant="outlined"
                    color="primary"
                    prepend-icon="mdi-account-plus"
                    data-testid="person-invite-cta"
                    @click="openInvite"
                >
                    {{ t('person.invite.cta') }}
                </v-btn>
                <v-btn
                    color="error"
                    variant="outlined"
                    data-testid="person-delete-button"
                    @click="confirmDelete = true"
                >
                    {{ t('common.delete') }}
                </v-btn>
            </div>

            <v-dialog v-model="inviteOpen" max-width="480">
                <v-card v-if="person !== null" data-testid="person-invite-modal">
                    <v-card-title>
                        {{
                            t('person.invite.modalTitle', {
                                name: `${person.given_name} ${person.family_name}`.trim(),
                            })
                        }}
                    </v-card-title>
                    <v-card-text>
                        <v-text-field
                            v-model="inviteEmail"
                            :label="t('person.invite.emailLabel')"
                            type="email"
                            density="compact"
                            data-testid="person-invite-email"
                        />
                        <v-select
                            v-model="inviteRole"
                            :label="t('person.invite.roleLabel')"
                            :items="[
                                { title: t('admin.members.role.user'), value: 'user' },
                                { title: t('admin.members.role.admin'), value: 'admin' },
                            ]"
                            density="compact"
                            hide-details
                            data-testid="person-invite-role"
                        />
                    </v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn variant="text" @click="inviteOpen = false">
                            {{ t('common.cancel') }}
                        </v-btn>
                        <v-btn
                            color="primary"
                            variant="flat"
                            :disabled="inviteEmail.trim().length === 0"
                            :loading="inviteMutation.isPending.value"
                            data-testid="person-invite-submit"
                            @click="submitInvite"
                        >
                            {{ t('person.invite.submit') }}
                        </v-btn>
                    </v-card-actions>
                </v-card>
            </v-dialog>

            <v-dialog v-model="confirmDelete" max-width="420">
                <v-card>
                    <v-card-title>{{ t('person.confirm.deleteTitle') }}</v-card-title>
                    <v-card-text>{{ t('person.confirm.deleteText') }}</v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn variant="text" @click="confirmDelete = false">
                            {{ t('common.cancel') }}
                        </v-btn>
                        <v-btn
                            color="error"
                            :loading="del.isPending.value"
                            data-testid="person-delete-confirm"
                            @click="remove"
                        >
                            {{ t('common.delete') }}
                        </v-btn>
                    </v-card-actions>
                </v-card>
            </v-dialog>
        </template>

        <PersonEdit
            v-else-if="editing && person !== null"
            mode="edit"
            :initial="{
                id: person.id,
                given_name: person.given_name,
                family_name: person.family_name,
                name_at_birth: person.name_at_birth,
                nickname: person.nickname,
                gender: person.gender,
                birth_date: person.birth_date ?? null,
                birth_place: person.birth_place,
                death_date: person.death_date ?? null,
                notes: person.notes,
                linked_user_id: person.linked_user_id ?? null,
            }"
            @saved="onSaved"
            @cancel="editing = false"
        />
    </section>
</template>
