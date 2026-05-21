<script setup lang="ts">
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'

import { type PersonInput, useCreatePerson, useUpdatePerson } from '@/api/hooks/persons'

interface Initial {
    id?: string
    given_name?: string
    family_name?: string
    nickname?: string
    gender?: string
    birth_date?: string | null
    birth_place?: string
    death_date?: string | null
    notes?: string
}

const props = defineProps<{
    initial?: Initial
    mode: 'create' | 'edit'
}>()

const emit = defineEmits<{
    (e: 'saved', id: string): void
    (e: 'cancel'): void
}>()

const { t } = useI18n()

// `reactive` (not `ref`) so v-model two-way binding works against the
// individual fields without unwrapping a giant `.value` per field.
const form = reactive<PersonInput>({
    given_name: props.initial?.given_name ?? '',
    family_name: props.initial?.family_name ?? '',
    nickname: props.initial?.nickname ?? '',
    gender: props.initial?.gender ?? '',
    birth_date: props.initial?.birth_date ?? null,
    birth_place: props.initial?.birth_place ?? '',
    death_date: props.initial?.death_date ?? null,
    notes: props.initial?.notes ?? '',
})

const create = useCreatePerson()
const update = useUpdatePerson()

async function submit(): Promise<void> {
    // `mutateAsync` rejects on validation/server errors; the global mutation
    // cache pushes an error toast so we only need to handle the happy path.
    if (props.mode === 'create') {
        const p = await create.mutateAsync(form)
        emit('saved', p.id)
        return
    }
    const id = props.initial?.id
    if (id === undefined) return
    const p = await update.mutateAsync({ id, input: form })
    emit('saved', p.id)
}
</script>

<template>
    <v-form data-testid="person-edit" @submit.prevent="submit">
        <v-text-field
            v-model="form.given_name"
            :label="t('person.fields.given_name')"
            required
            autocomplete="given-name"
            data-testid="person-given-name"
        />
        <v-text-field
            v-model="form.family_name"
            :label="t('person.fields.family_name')"
            autocomplete="family-name"
            data-testid="person-family-name"
        />
        <v-text-field
            v-model="form.birth_date"
            :label="t('person.fields.birth_date')"
            type="date"
            data-testid="person-birth-date"
        />
        <v-text-field v-model="form.gender" :label="t('person.fields.gender')" data-testid="person-gender" />
        <v-textarea
            v-model="form.notes"
            :label="t('person.fields.notes')"
            rows="3"
            auto-grow
            data-testid="person-notes"
        />
        <div class="d-flex justify-end ga-2 mt-2">
            <v-btn variant="text" @click="emit('cancel')">{{ t('common.cancel') }}</v-btn>
            <v-btn
                type="submit"
                color="primary"
                :loading="create.isPending.value || update.isPending.value"
                data-testid="person-submit"
            >
                {{ t('common.save') }}
            </v-btn>
        </div>
    </v-form>
</template>
