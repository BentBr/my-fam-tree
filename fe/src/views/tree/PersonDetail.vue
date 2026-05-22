<script setup lang="ts">
import { computed, ref, toRef } from 'vue'
import { useI18n } from 'vue-i18n'

import { useDeletePerson, useGetPerson, useListPersons } from '@/api/hooks/persons'
import { useActiveFamilyStore } from '@/stores/activeFamily'

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

// owner+admin can edit; user role is read-only. `null` happens during the
// brief window after sign-in before the active-family hydrates — render
// read-only until we know.
const canEdit = computed(() => {
    const role = family.activeFamily?.role ?? null
    return role === 'owner' || role === 'admin'
})

const editing = ref(false)
const confirmDelete = ref(false)

const person = computed(() => personQ.data.value ?? list.data.value?.find((p) => p.id === props.personId) ?? null)

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
            <h3 v-if="person" class="text-h6" data-testid="person-detail-title">
                {{ person.given_name }} {{ person.family_name }}
            </h3>
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

            <!-- Contact section. Renders empty fields as em-dash, matching
                 the profile section above so the drawer stays scannable. -->
            <h4 class="text-subtitle-1 mb-2">{{ t('person.sections.contact') }}</h4>
            <v-list density="compact" class="mb-2" data-testid="person-contact-list">
                <v-list-item data-testid="person-field-email">
                    <v-list-item-title>{{ t('person.fields.email') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.email || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-phone">
                    <v-list-item-title>{{ t('person.fields.phone') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.phone || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-street">
                    <v-list-item-title>{{ t('person.fields.street') }}</v-list-item-title>
                    <v-list-item-subtitle>
                        {{ [person.street, person.house_number].filter(Boolean).join(' ') || '—' }}
                    </v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-city">
                    <v-list-item-title>{{ t('person.fields.city') }}</v-list-item-title>
                    <v-list-item-subtitle>
                        {{ [person.zip, person.city].filter(Boolean).join(' ') || '—' }}
                    </v-list-item-subtitle>
                </v-list-item>
                <v-list-item data-testid="person-field-country">
                    <v-list-item-title>{{ t('person.fields.country') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.country || '—' }}</v-list-item-subtitle>
                </v-list-item>
            </v-list>

            <v-divider class="my-3" />

            <PersonRelations :person-id="props.personId" :can-edit="canEdit" @changed="onRelationsChanged" />

            <v-divider class="my-3" />

            <div v-if="canEdit" class="d-flex ga-2">
                <v-btn variant="outlined" data-testid="person-edit-button" @click="editing = true">
                    {{ t('common.edit') }}
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
                email: person.email,
                phone: person.phone,
                street: person.street,
                house_number: person.house_number,
                zip: person.zip,
                city: person.city,
                country: person.country,
                linked_user_id: person.linked_user_id ?? null,
            }"
            @saved="onSaved"
            @cancel="editing = false"
        />
    </section>
</template>
