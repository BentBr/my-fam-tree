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

// Single-use tokens MUST only be consumed once. A component-scoped ref
// (`const consumed = ref(false)`) is NOT enough — Vite dev HMR can
// re-mount the same route in a new component instance, and CI surfaces
// double-fires that local dev doesn't reproduce. We key the dedup on
// the token itself in sessionStorage so any subsequent mount with the
// same token short-circuits before hitting `/auth/consume`. The entry
// is cleared on `auth.logout()` (it's under `my-fam-tree:`); a fresh
// sign-in mints a new token, so stale-token leakage isn't a concern.

onMounted(async () => {
    const token = String(route.query['token'] ?? '')
    if (token === '') {
        status.value = 'error'
        return
    }
    const dedupeKey = `my-fam-tree:consumed:${token}`
    if (sessionStorage.getItem(dedupeKey) !== null) {
        // Already consumed in a previous mount of this same token; the
        // first mount's success path already redirected. If we got
        // re-mounted before the navigation settled, finish the redirect
        // here rather than re-firing the now-invalid POST.
        status.value = 'ok'
        await safeReplace('/tree')
        return
    }
    sessionStorage.setItem(dedupeKey, '1')
    try {
        await mutation.mutateAsync(token)
    } catch {
        // Roll back the dedup marker so a manual retry (refresh) can
        // re-attempt with the same URL. The token is single-use server-
        // side anyway; the rollback only matters for "page mounted but
        // network blew up" which won't actually allow re-consume.
        sessionStorage.removeItem(dedupeKey)
        status.value = 'error'
        return
    }
    // Auth is established — flip to success BEFORE attempting the
    // post-consume navigation. If a route guard redirects the push
    // (e.g., the family-active guard bounces the user to
    // `/families/create`), router.replace returns a NavigationFailure
    // rather than throwing; the previous combined try-catch would
    // still have surfaced the error UI on any throw inside the push,
    // even though the user is signed in. Keep the success state and
    // let the router land where it lands.
    status.value = 'ok'
    await safeReplace('/tree')
})

async function safeReplace(to: string): Promise<void> {
    try {
        await router.replace(to)
    } catch {
        // Aborted / duplicate / guard-redirected navigations are not
        // failures of the consume; the auth store is already authed
        // and the next user interaction will pick up the right route.
    }
}
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
