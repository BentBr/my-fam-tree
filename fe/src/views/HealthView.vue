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
// Show one decimal so sub-millisecond DB pings ("0.3 ms") don't render
// as a misleading "0 ms". The BE returns float ms; we just format
// here. Network panel's reported request time will always be larger
// than these in-handler values — TLS, Nginx, geographic distance,
// actix's pre-handler middleware all sit outside this measurement.
function fmtMs(ms: number): string {
    if (!Number.isFinite(ms)) return '0'
    if (ms < 10) return ms.toFixed(1)
    return Math.round(ms).toString()
}
// Worker lease: the BE probes Redis for the reminder-leader key. The
// chip flips green when held, red when the lease has expired (worker
// is down or hasn't been deployed yet). No latency dimension — it's a
// pure liveness boolean.
const workerOk = computed(() => data.value?.data.worker_ok ?? false)

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
const workerColor = computed<'success' | 'error'>(() => (workerOk.value ? 'success' : 'error'))
const dbText = computed(() =>
    dbOk.value ? t('health.dbLatency', { ms: fmtMs(dbLatency.value) }) : t('health.dbDown'),
)
const serverText = computed(() => t('health.serverLatency', { ms: fmtMs(serverLatency.value) }))
const workerText = computed(() => (workerOk.value ? t('health.workerAlive') : t('health.workerDown')))
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
                <!-- Three liveness chips side-by-side: DB on the left,
                     worker leader-lease in the middle, full handler
                     duration on the right. `server_duration_ms` is
                     always >= `db_latency_ms` (the DB probe runs
                     inside the handler timer); when the gap between the
                     two is large it points at framework / serialisation
                     overhead rather than the DB. The worker chip has
                     no latency dimension — it's a pure boolean from
                     the Redis lease check. -->
                <div class="d-flex flex-wrap ga-2 mb-3">
                    <v-chip :color="dbColor" variant="flat" data-testid="health-db">
                        <v-icon start icon="database" />
                        {{ dbText }}
                    </v-chip>
                    <v-chip :color="workerColor" variant="flat" data-testid="health-worker">
                        <v-icon start icon="cog" />
                        {{ workerText }}
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
