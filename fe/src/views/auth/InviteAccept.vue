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
type Status = 'pending' | 'ok' | 'mismatch' | 'error'
const status = ref<Status>('pending')
const processed = ref(false)

// The invite token is itself the auth factor server-side: anonymous
// callers get a fresh user keyed on `invite.email`, signed-in callers
// are validated against the invite's email. Either way we POST the
// token in one round-trip — no sessionStorage stashing, no detour
// through /auth/sign-in.
onMounted(async () => {
    if (processed.value) return
    processed.value = true
    const token = String(route.query['token'] ?? '')
    if (token === '') {
        status.value = 'error'
        return
    }
    try {
        const res = await accept.mutateAsync(token)
        if (res !== undefined) {
            // Hydrate the auth store from the BE's response so the
            // freshly-issued cookie is reflected in the SPA's reactive
            // state without a refresh-token round-trip.
            await auth.hydrate()
            // CRITICAL: select the newly-joined family before navigating,
            // otherwise the family guard sees activeFamilyId === null and
            // bounces /health → /families/pick.
            family.setActive(res.data.family.id as FamilyId)
        }
        status.value = 'ok'
        await nextTick()
        await router.push('/tree')
    } catch (err) {
        // 422 invite_email_mismatch is surfaced as a `Validation` ApiError
        // — the FE-readable hint lives inside `body.fields[].code`. We
        // check both the top-level code and the field-violation code so
        // a future BE refactor that promotes mismatch to a dedicated
        // variant still works without a FE change.
        const e = err as
            | { code?: string; body?: { fields?: { code?: string }[] | null } }
            | undefined
        const topCode = e?.code
        const fieldCodes = e?.body?.fields ?? []
        const mismatched =
            topCode === 'invite_email_mismatch' ||
            fieldCodes.some((f) => f?.code === 'validation.invite_email_mismatch')
        status.value = mismatched ? 'mismatch' : 'error'
    }
})

async function signOutAndRetry(): Promise<void> {
    const token = String(route.query['token'] ?? '')
    await auth.logout()
    // Re-enter this view with the same token. New (anonymous) session
    // hits the invite-as-auth code path on the BE.
    processed.value = false
    status.value = 'pending'
    await router.replace({ path: '/invite/accept', query: { token } })
}
</script>

<template>
    <v-card class="pa-6 text-center" data-testid="invite-card">
        <v-progress-circular v-if="status === 'pending'" indeterminate color="primary" data-testid="invite-pending" />
        <p v-if="status === 'pending'" class="mt-4">{{ t('invite.pending') }}</p>
        <v-alert v-else-if="status === 'mismatch'" type="warning" data-testid="invite-mismatch">
            {{ t('invite.mismatch') }}
            <template #append>
                <v-btn variant="text" data-testid="invite-mismatch-signout" @click="signOutAndRetry">
                    {{ t('auth.signOut') }}
                </v-btn>
            </template>
        </v-alert>
        <v-alert v-else-if="status === 'error'" type="error" data-testid="invite-error">
            {{ t('invite.error') }}
        </v-alert>
    </v-card>
</template>
