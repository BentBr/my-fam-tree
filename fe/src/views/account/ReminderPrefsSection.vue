<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { type ReminderPrefs, useReminderPrefs, useSaveReminderPrefs } from '@/api/hooks/reminders'

const { t } = useI18n()
const query = useReminderPrefs()
const save = useSaveReminderPrefs()

const form = ref<ReminderPrefs>({
    emails_enabled: false,
    remind_birthdays: true,
    remind_anniversaries: true,
    favourites_only: false,
    lead_days: 7,
})

// Sync the editable copy when the query resolves / refetches.
watch(
    () => query.data.value,
    (prefs) => {
        if (prefs !== undefined) form.value = { ...prefs }
    },
    { immediate: true },
)

async function onSave(): Promise<void> {
    await save.mutateAsync({ ...form.value })
}
</script>

<template>
    <section data-testid="reminder-prefs">
        <v-card-subtitle class="text-h6 px-0">{{ t('reminderPrefs.heading') }}</v-card-subtitle>
        <p class="text-body-2 mb-2">{{ t('reminderPrefs.subtitle') }}</p>

        <v-switch
            v-model="form.emails_enabled"
            color="primary"
            density="compact"
            hide-details
            :label="t('reminderPrefs.emailsEnabled')"
            data-testid="reminder-emails-enabled"
        />

        <div :class="{ 'reminder-disabled': !form.emails_enabled }">
            <v-switch
                v-model="form.remind_birthdays"
                :disabled="!form.emails_enabled"
                color="primary"
                density="compact"
                hide-details
                :label="t('reminderPrefs.birthdays')"
                data-testid="reminder-birthdays"
            />
            <v-switch
                v-model="form.remind_anniversaries"
                :disabled="!form.emails_enabled"
                color="primary"
                density="compact"
                hide-details
                :label="t('reminderPrefs.anniversaries')"
                data-testid="reminder-anniversaries"
            />
            <v-switch
                v-model="form.favourites_only"
                :disabled="!form.emails_enabled"
                color="primary"
                density="compact"
                hide-details
                :label="t('reminderPrefs.favouritesOnly')"
                data-testid="reminder-favourites-only"
            />
            <v-slider
                v-model="form.lead_days"
                :disabled="!form.emails_enabled"
                :min="0"
                :max="21"
                :step="1"
                thumb-label="always"
                class="mt-6"
                :label="t('reminderPrefs.leadDays')"
                data-testid="reminder-lead-days"
            />
            <p class="text-caption text-medium-emphasis">
                {{
                    form.lead_days === 0
                        ? t('reminderPrefs.leadDayOf')
                        : t('reminderPrefs.leadHint', { n: form.lead_days })
                }}
            </p>
        </div>

        <v-btn
            color="primary"
            :loading="save.isPending.value"
            block
            class="mt-2"
            data-testid="reminder-save"
            @click="onSave"
        >
            {{ t('common.save') }}
        </v-btn>
    </section>
</template>

<style scoped>
.reminder-disabled {
    opacity: 0.5;
}
</style>
