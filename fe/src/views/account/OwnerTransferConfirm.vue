<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { useConfirmOwnerTransfer } from '@/api/hooks/owner_transfer'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const confirmM = useConfirmOwnerTransfer()

type Status = 'pending' | 'success-one' | 'success-both' | 'error'
const status = ref<Status>('pending')

// Single-use guard — Vue's dev double-mount semantics + StrictMode would
// otherwise burn the token twice and surface a spurious error. Same
// pattern as ConsumeView for the magic-link flow + EmailChangeConsumeView.
const processed = ref(false)

onMounted(async () => {
    if (processed.value) return
    processed.value = true
    const raw = route.query['token']
    const token = typeof raw === 'string' ? raw : ''
    if (token === '') {
        status.value = 'error'
        return
    }
    try {
        const res = await confirmM.mutateAsync(token)
        status.value = res.from_confirmed && res.to_confirmed ? 'success-both' : 'success-one'
        // Strip the token from the URL. Safe to mutate the route here — this
        // page is a one-shot terminal view, not a drawer with a route-watcher.
        const rest = { ...route.query }
        delete rest['token']
        void router.replace({ query: rest })
    } catch {
        status.value = 'error'
    }
})
</script>

<template>
    <section class="confirm-page pa-6" data-testid="owner-transfer-confirm">
        <h2 class="text-h6 mb-3">{{ t('admin.transfer.confirmPage.title') }}</h2>
        <v-alert v-if="status === 'pending'" type="info" variant="tonal" data-testid="owner-transfer-pending">
            {{ t('common.loading') }}
        </v-alert>
        <v-alert
            v-else-if="status === 'success-one'"
            type="success"
            variant="tonal"
            data-testid="owner-transfer-success-one"
        >
            {{ t('admin.transfer.confirmPage.successOne') }}
        </v-alert>
        <v-alert
            v-else-if="status === 'success-both'"
            type="success"
            variant="tonal"
            data-testid="owner-transfer-success-both"
        >
            {{ t('admin.transfer.confirmPage.successBoth') }}
        </v-alert>
        <v-alert v-else type="error" variant="tonal" data-testid="owner-transfer-error">
            {{ t('admin.transfer.confirmPage.error') }}
        </v-alert>
    </section>
</template>

<style scoped>
.confirm-page {
    width: 100%;
}
</style>
