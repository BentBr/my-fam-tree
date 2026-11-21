<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type { PersonView } from '@/api/types'

defineProps<{
    person: PersonView
}>()

const { t } = useI18n()
</script>

<template>
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
</template>
