<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { useLocaleStore, type SupportedLocale } from '@/stores/locale'

const store = useLocaleStore()
const { t } = useI18n()

function onChange(value: unknown): void {
    if (value === 'en' || value === 'de') store.set(value as SupportedLocale)
}
</script>

<template>
    <v-select
        :model-value="store.locale"
        :items="[
            { value: 'en', title: t('language.en') },
            { value: 'de', title: t('language.de') },
        ]"
        :label="t('language.label')"
        item-value="value"
        item-title="title"
        density="compact"
        hide-details
        style="max-width: 140px"
        @update:model-value="onChange"
    />
</template>
