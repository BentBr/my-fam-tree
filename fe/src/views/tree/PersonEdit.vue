<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { useCreatePerson, useUpdatePerson } from '@/api/hooks/persons'

interface Initial {
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
// `reactive` with concrete `string` types (no `undefined`) so v-model
// binds work cleanly against Vuetify components under
// `exactOptionalPropertyTypes`. The wire payload is built from this in
// `submit()`; empty strings collapse to "no value" naturally on the API
// side. PersonInput is a wider typescript view; we narrow here.
interface FormShape {
    given_name: string
    family_name: string
    name_at_birth: string
    nickname: string
    gender: string
    birth_date: string | null
    birth_place: string
    death_date: string | null
    notes: string
    email: string
    phone: string
    street: string
    house_number: string
    zip: string
    city: string
    country: string
}
const form = reactive<FormShape>({
    given_name: props.initial?.given_name ?? '',
    family_name: props.initial?.family_name ?? '',
    name_at_birth: props.initial?.name_at_birth ?? '',
    nickname: props.initial?.nickname ?? '',
    gender: props.initial?.gender ?? '',
    birth_date: props.initial?.birth_date ?? null,
    birth_place: props.initial?.birth_place ?? '',
    death_date: props.initial?.death_date ?? null,
    notes: props.initial?.notes ?? '',
    email: props.initial?.email ?? '',
    phone: props.initial?.phone ?? '',
    street: props.initial?.street ?? '',
    house_number: props.initial?.house_number ?? '',
    zip: props.initial?.zip ?? '',
    city: props.initial?.city ?? '',
    country: props.initial?.country ?? '',
})

// "Deceased" is a UI-only checkbox; it gates whether the death_date
// field is shown. On uncheck we clear `form.death_date` so the wire
// payload doesn't carry a stale date the user can't see.
const deceased = ref(form.death_date !== null && form.death_date !== '')
watch(deceased, (v) => {
    if (!v) form.death_date = null
})

// `email` is read-only when the person is linked to a user (the server
// rewrites the column from `users.email` on every write, so an editable
// field would mislead the user into thinking their input mattered).
const emailReadOnly = computed(
    () => typeof props.initial?.linked_user_id === 'string' && props.initial.linked_user_id !== '',
)

// Canonical gender options as v-combobox items. `combobox` (not `select`)
// keeps the field free-text-capable: the user can type something the
// dropdown doesn't list and it goes to the backend verbatim. The items
// are plain strings (localized labels) — v-combobox then binds the
// model directly to a string, which matches `PersonInput.gender`.
const genderOptions = computed(() => [t('person.gender.male'), t('person.gender.female'), t('person.gender.diverse')])

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
        <!-- Profile fields — every PersonView name + identity field is
             editable here, in the same top-to-bottom order as the view's
             Profile section so the user's reading path matches. -->
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
            v-model="form.name_at_birth"
            :label="t('person.fields.name_at_birth')"
            data-testid="person-name-at-birth"
        />
        <v-text-field v-model="form.nickname" :label="t('person.fields.nickname')" data-testid="person-nickname" />
        <v-text-field
            v-model="form.birth_date"
            :label="t('person.fields.birth_date')"
            type="date"
            data-testid="person-birth-date"
        />
        <v-text-field
            v-model="form.birth_place"
            :label="t('person.fields.birth_place')"
            data-testid="person-birth-place"
        />
        <v-checkbox
            v-model="deceased"
            :label="t('person.fields.deceased')"
            density="compact"
            hide-details
            data-testid="person-deceased"
        />
        <v-text-field
            v-if="deceased"
            v-model="form.death_date"
            :label="t('person.fields.death_date')"
            type="date"
            data-testid="person-death-date"
        />
        <v-combobox
            v-model="form.gender"
            :items="genderOptions"
            :label="t('person.fields.gender')"
            data-testid="person-gender"
        />
        <v-textarea
            v-model="form.notes"
            :label="t('person.fields.notes')"
            rows="3"
            auto-grow
            data-testid="person-notes"
        />

        <v-divider class="my-3" />
        <h4 class="text-subtitle-1 mb-2">{{ t('person.sections.contact') }}</h4>

        <!-- Email + phone — email is read-only when linked to a user.
             The hint surfaces *why* the field is locked. The server
             enforces the same rule (overrides the column from
             users.email regardless of the body), so this is purely
             UX-side; security comes from the backend. -->
        <v-text-field
            v-model="form.email"
            :label="t('person.fields.email')"
            :readonly="emailReadOnly"
            :persistent-hint="emailReadOnly"
            :hint="emailReadOnly ? t('person.hints.emailFromLinkedUser') : ''"
            type="email"
            autocomplete="email"
            data-testid="person-email"
        />
        <v-text-field
            v-model="form.phone"
            :label="t('person.fields.phone')"
            type="tel"
            autocomplete="tel"
            data-testid="person-phone"
        />

        <h4 class="text-subtitle-1 mb-2 mt-4">{{ t('person.sections.address') }}</h4>

        <v-row dense>
            <v-col cols="9">
                <v-text-field
                    v-model="form.street"
                    :label="t('person.fields.street')"
                    autocomplete="street-address"
                    data-testid="person-street"
                />
            </v-col>
            <v-col cols="3">
                <v-text-field
                    v-model="form.house_number"
                    :label="t('person.fields.house_number')"
                    data-testid="person-house-number"
                />
            </v-col>
        </v-row>
        <v-row dense>
            <v-col cols="3">
                <v-text-field
                    v-model="form.zip"
                    :label="t('person.fields.zip')"
                    autocomplete="postal-code"
                    data-testid="person-zip"
                />
            </v-col>
            <v-col cols="5">
                <v-text-field
                    v-model="form.city"
                    :label="t('person.fields.city')"
                    autocomplete="address-level2"
                    data-testid="person-city"
                />
            </v-col>
            <v-col cols="4">
                <v-text-field
                    v-model="form.country"
                    :label="t('person.fields.country')"
                    autocomplete="country-name"
                    data-testid="person-country"
                />
            </v-col>
        </v-row>

        <div class="d-flex justify-end ga-2 mt-2">
            <v-btn variant="text" data-testid="person-edit-cancel" @click="emit('cancel')">
                {{ t('common.cancel') }}
            </v-btn>
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
