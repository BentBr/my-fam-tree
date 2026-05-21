<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { useConfirmEmailChange } from '@/api/hooks/users'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const confirm = useConfirmEmailChange()
const status = ref<'pending' | 'ok' | 'error'>('pending')

// Single-use token guard — Vue's dev double-mount semantics would otherwise
// burn the token twice and surface a spurious error to the user. Same pattern
// as ConsumeView for the magic-link flow.
const processed = ref(false)

onMounted(async () => {
    if (processed.value) return
    processed.value = true
    const token = String(route.query['token'] ?? '')
    if (token === '') {
        status.value = 'error'
        return
    }
    try {
        await confirm.mutateAsync(token)
        status.value = 'ok'
        await router.replace('/account')
    } catch {
        status.value = 'error'
    }
})
</script>

<template>
    <v-card class="pa-6 text-center" data-testid="email-change-card">
        <template v-if="status === 'pending'">
            <v-progress-circular indeterminate color="primary" data-testid="email-change-pending-spinner" />
            <p class="mt-4">{{ t('account.email.confirmPending') }}</p>
        </template>
        <v-alert v-else-if="status === 'error'" type="error" data-testid="email-change-error">
            {{ t('account.email.confirmError') }}
        </v-alert>
    </v-card>
</template>
