<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { useUpdateMe } from '@/api/hooks/users'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore, type SupportedLocale } from '@/stores/locale'

const store = useLocaleStore()
const auth = useAuthStore()
const update = useUpdateMe()
const { t } = useI18n()

function onChange(value: unknown): void {
    if (value !== 'en' && value !== 'de') return
    const next: SupportedLocale = value
    // Optimistic local update so the UI flips immediately.
    store.set(next)
    // Persist to the backend when the caller has a session. Anonymous users
    // (login screen) keep the locale local-only — the next sign-in then syncs
    // it via applyClaimsPayload.
    if (auth.status === 'authenticated') {
        update.mutate({ locale: next })
    }
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
