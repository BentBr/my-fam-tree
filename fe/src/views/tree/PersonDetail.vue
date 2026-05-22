<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { useDeletePerson, useListPersons } from '@/api/hooks/persons'
import { useAddParentLink, useCreatePartnership } from '@/api/hooks/relationships'

import PersonEdit from './PersonEdit.vue'

const props = defineProps<{
    personId: string
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'changed'): void
}>()

const { t } = useI18n()
const list = useListPersons()
const del = useDeletePerson()
const addParent = useAddParentLink()
const createPartner = useCreatePartnership()

const editing = ref(false)
const parentToAdd = ref<string | null>(null)
const partnerToAdd = ref<string | null>(null)
// Default: biological — that's the overwhelming majority of family-tree
// edges and matches what every backend test seeds. The dropdown still
// lets the user override before submit.
const parentKind = ref<'biological' | 'legal' | 'adoptive' | 'step' | 'social'>('biological')
// No default for partnership kind — civil unions vs marriages have
// different legal/historical weight and we'd rather force a deliberate
// pick than mis-tag the user's first attempt.
const partnerKind = ref<'marriage' | 'civil_union' | 'partnership' | null>(null)
const confirmDelete = ref(false)

const parentKindOptions = computed(() => [
    { value: 'biological', title: t('person.parentKind.biological') },
    { value: 'legal', title: t('person.parentKind.legal') },
    { value: 'adoptive', title: t('person.parentKind.adoptive') },
    { value: 'step', title: t('person.parentKind.step') },
    { value: 'social', title: t('person.parentKind.social') },
])

const partnerKindOptions = computed(() => [
    { value: 'marriage', title: t('person.partnerKind.marriage') },
    { value: 'civil_union', title: t('person.partnerKind.civil_union') },
    { value: 'partnership', title: t('person.partnerKind.partnership') },
])

const person = computed(() => list.data.value?.find((p) => p.id === props.personId) ?? null)

// Candidates for "add parent" / "add partner" — everyone in the family except
// the currently-viewed person. v-select items need `{value, title}` pairs.
const otherItems = computed(() =>
    (list.data.value ?? [])
        .filter((p) => p.id !== props.personId)
        .map((p) => ({
            value: p.id,
            title: `${p.given_name} ${p.family_name}`.trim() || p.given_name,
        })),
)

async function remove(): Promise<void> {
    await del.mutateAsync(props.personId)
    confirmDelete.value = false
    emit('changed')
    emit('close')
}

async function linkParent(): Promise<void> {
    const pid = parentToAdd.value
    if (pid === null || pid === '') return
    await addParent.mutateAsync({
        child_id: props.personId,
        parent_id: pid,
        kind: parentKind.value,
    })
    parentToAdd.value = null
    parentKind.value = 'biological'
    emit('changed')
}

async function linkPartner(): Promise<void> {
    const pid = partnerToAdd.value
    const kind = partnerKind.value
    if (pid === null || pid === '' || kind === null) return
    await createPartner.mutateAsync({
        partner_a_id: props.personId,
        partner_b_id: pid,
        kind,
    })
    partnerToAdd.value = null
    partnerKind.value = null
    emit('changed')
}

function onSaved(): void {
    editing.value = false
    emit('changed')
}
</script>

<template>
    <section class="pa-4" data-testid="person-detail">
        <header class="d-flex align-center justify-space-between mb-3">
            <h3 v-if="person" class="text-h6">{{ person.given_name }} {{ person.family_name }}</h3>
            <v-btn icon="x" variant="text" size="small" data-testid="person-detail-close" @click="emit('close')" />
        </header>

        <template v-if="!editing && person !== null">
            <v-list density="compact" class="mb-2">
                <v-list-item>
                    <v-list-item-title>{{ t('person.fields.birth_date') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.birth_date ?? '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item>
                    <v-list-item-title>{{ t('person.fields.gender') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.gender || '—' }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item v-if="person.notes">
                    <v-list-item-title>{{ t('person.fields.notes') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ person.notes }}</v-list-item-subtitle>
                </v-list-item>
            </v-list>

            <v-divider class="my-3" />

            <div class="mb-2">
                <v-select
                    v-model="parentToAdd"
                    :items="otherItems"
                    item-value="value"
                    item-title="title"
                    :label="t('person.actions.addParent')"
                    clearable
                    density="comfortable"
                    data-testid="person-add-parent"
                />
                <v-select
                    v-model="parentKind"
                    :items="parentKindOptions"
                    item-value="value"
                    item-title="title"
                    :label="t('person.fields.parentKind')"
                    density="comfortable"
                    data-testid="person-add-parent-kind"
                />
                <v-btn
                    block
                    color="primary"
                    variant="tonal"
                    :disabled="parentToAdd === null || parentToAdd === ''"
                    :loading="addParent.isPending.value"
                    data-testid="person-add-parent-submit"
                    @click="linkParent"
                >
                    {{ t('common.add') }}
                </v-btn>
            </div>

            <div class="mb-2">
                <v-select
                    v-model="partnerToAdd"
                    :items="otherItems"
                    item-value="value"
                    item-title="title"
                    :label="t('person.actions.addPartner')"
                    clearable
                    density="comfortable"
                    data-testid="person-add-partner"
                />
                <v-select
                    v-model="partnerKind"
                    :items="partnerKindOptions"
                    item-value="value"
                    item-title="title"
                    :label="t('person.fields.partnerKind')"
                    density="comfortable"
                    clearable
                    data-testid="person-add-partner-kind"
                />
                <v-btn
                    block
                    color="primary"
                    variant="tonal"
                    :disabled="partnerToAdd === null || partnerToAdd === '' || partnerKind === null"
                    :loading="createPartner.isPending.value"
                    data-testid="person-add-partner-submit"
                    @click="linkPartner"
                >
                    {{ t('common.add') }}
                </v-btn>
            </div>

            <v-divider class="my-3" />

            <div class="d-flex ga-2">
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
                nickname: person.nickname,
                gender: person.gender,
                birth_date: person.birth_date ?? null,
                birth_place: person.birth_place,
                death_date: person.death_date ?? null,
                notes: person.notes,
            }"
            @saved="onSaved"
            @cancel="editing = false"
        />
    </section>
</template>
