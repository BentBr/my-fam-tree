<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { useHealth } from '@/api/hooks/health'

const { t } = useI18n()
const { data, isLoading, error } = useHealth()

const version = computed(() => data.value?.data.version ?? '')
const requestId = computed(() => data.value?.meta?.request_id ?? '')
const dbOk = computed(() => data.value?.data.db_ok ?? false)
const dbLatency = computed(() => data.value?.data.db_latency_ms ?? 0)
const serverLatency = computed(() => data.value?.data.server_duration_ms ?? 0)

// Latency thresholds: < 100 ms green, < 200 ms yellow, otherwise red. An
// unreachable DB is always red regardless of the measured time.
function colourFor(ms: number, ok = true): 'success' | 'warning' | 'error' {
    if (!ok) return 'error'
    if (ms < 100) return 'success'
    if (ms < 200) return 'warning'
    return 'error'
}
const dbColor = computed(() => colourFor(dbLatency.value, dbOk.value))
const serverColor = computed(() => colourFor(serverLatency.value))
const dbText = computed(() => (dbOk.value ? t('health.dbLatency', { ms: dbLatency.value }) : t('health.dbDown')))
const serverText = computed(() => t('health.serverLatency', { ms: serverLatency.value }))
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
                <!-- Two latency chips side-by-side: DB on the left, full
                     handler duration on the right. `server_duration_ms`
                     is always >= `db_latency_ms` (the DB probe runs
                     inside the handler timer); when the gap between the
                     two is large it points at framework / serialisation
                     overhead rather than the DB. -->
                <div class="d-flex flex-wrap ga-2 mb-3">
                    <v-chip :color="dbColor" variant="flat" data-testid="health-db">
                        <v-icon start icon="database" />
                        {{ dbText }}
                    </v-chip>
                    <v-chip :color="serverColor" variant="flat" data-testid="health-server">
                        <v-icon start icon="server" />
                        {{ serverText }}
                    </v-chip>
                </div>
                <v-list density="compact">
                    <v-list-item :prepend-icon="'tag'" :title="t('health.version', { version })" />
                    <v-list-item :prepend-icon="'fingerprint'" :title="t('health.requestId', { id: requestId })" />
                </v-list>
            </div>
        </v-card-text>
    </v-card>
</template>
