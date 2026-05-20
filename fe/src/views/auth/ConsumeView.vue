<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { useConsumeMagicLink } from '@/api/hooks/auth'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const mutation = useConsumeMagicLink()
const status = ref<'pending' | 'ok' | 'error'>('pending')

onMounted(async () => {
    const token = String(route.query['token'] ?? '')
    if (token === '') {
        status.value = 'error'
        return
    }
    try {
        await mutation.mutateAsync(token)
        status.value = 'ok'
        await router.replace('/tree')
    } catch {
        status.value = 'error'
    }
})
</script>

<template>
    <v-card class="pa-6 text-center" data-testid="consume-card">
        <v-progress-circular v-if="status === 'pending'" indeterminate color="primary" data-testid="consume-pending" />
        <p v-if="status === 'pending'" class="mt-4">{{ t('auth.consume.pending') }}</p>

        <v-alert v-else-if="status === 'error'" type="error" data-testid="consume-error">
            {{ t('auth.consume.error') }}
            <template #append>
                <v-btn to="/auth/sign-in" variant="text">{{ t('auth.consume.tryAgain') }}</v-btn>
            </template>
        </v-alert>
    </v-card>
</template>
