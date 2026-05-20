<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { useHealth } from '@/api/hooks/health'

const { t } = useI18n()
const { data, isLoading, error } = useHealth()
const version = computed(() => data.value?.data.version ?? '')
const requestId = computed(() => data.value?.meta?.request_id ?? '')
</script>

<template>
    <v-card>
        <v-card-title>{{ t('health.title') }}</v-card-title>
        <v-card-text>
            <div v-if="isLoading" data-testid="health-loading">
                <v-progress-linear indeterminate color="primary" class="mb-3" />
                {{ t('health.loading') }}
            </div>
            <v-alert v-else-if="error" type="error" data-testid="health-error">
                {{ t('health.error') }}
            </v-alert>
            <div v-else data-testid="health-ok">
                <v-alert type="success" variant="tonal" class="mb-3">{{ t('health.ok') }}</v-alert>
                <v-list density="compact">
                    <v-list-item :prepend-icon="'tag'" :title="t('health.version', { version })" />
                    <v-list-item :prepend-icon="'fingerprint'" :title="t('health.requestId', { id: requestId })" />
                </v-list>
            </div>
        </v-card-text>
    </v-card>
</template>
