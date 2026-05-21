<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { useAcceptInvite } from '@/api/hooks/families'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import type { FamilyId } from '@/types/brand'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const auth = useAuthStore()
const family = useActiveFamilyStore()
const accept = useAcceptInvite()
const status = ref<'pending' | 'ok' | 'error'>('pending')
const processed = ref(false)

onMounted(async () => {
    if (processed.value) {
        return
    }
    processed.value = true
    const token = String(route.query['token'] ?? '')
    if (token === '') {
        status.value = 'error'
        return
    }
    if (auth.status === 'anonymous') {
        // Stash the token so the user can complete sign-in then come back.
        sessionStorage.setItem('my-family:inviteToken', token)
        await router.replace('/auth/sign-in')
        return
    }
    try {
        const res = await accept.mutateAsync(token)
        if (res !== undefined) {
            // CRITICAL: select the newly-joined family before navigating,
            // otherwise the family guard sees activeFamilyId === null and
            // bounces /health → /families/pick.
            family.setActive(res.data.family.id as FamilyId)
        }
        status.value = 'ok'
        await nextTick()
        await router.push('/health')
    } catch {
        status.value = 'error'
    }
})
</script>

<template>
    <v-card class="pa-6 text-center" data-testid="invite-card">
        <v-progress-circular v-if="status === 'pending'" indeterminate color="primary" data-testid="invite-pending" />
        <p v-if="status === 'pending'" class="mt-4">{{ t('invite.pending') }}</p>
        <v-alert v-else-if="status === 'error'" type="error" data-testid="invite-error">
            {{ t('invite.error') }}
        </v-alert>
    </v-card>
</template>
