<script setup lang="ts">
/**
 * Right-edge account control of the AppBar.
 *
 * One activator, one menu, both surfaces. The dropdown trigger is
 * always the same avatar-button — what *opens* underneath is the
 * only thing that swaps with auth state:
 *
 *   logged out → Login (primary), Register   (both → /auth/sign-in)
 *   logged in  → Account (→ /account), Sign out
 *
 * Single component for both shapes keeps the AppBar's right edge
 * geometrically stable across sign-in / sign-out transitions, and
 * means the public + authenticated chrome share the same control
 * (per the unified-navbar decision in the plan).
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useMe } from '@/api/hooks/users'
import DefaultAvatar from '@/components/common/DefaultAvatar.vue'
import { useAuthStore } from '@/stores/auth'

const { t } = useI18n()
const auth = useAuthStore()
const router = useRouter()

// Authenticated: read display name + avatar for the activator;
// anonymous callers get a generic placeholder via DefaultAvatar's
// own empty-name handling.
const me = useMe()
const isAuthed = computed(() => auth.status === 'authenticated')
const avatarUrl = computed(() => (isAuthed.value ? (me.data.value?.avatar_url ?? null) : null))
const displayName = computed(() => (isAuthed.value ? (me.data.value?.display_name ?? auth.user?.email ?? '') : ''))
const activatorLabel = computed(() => (isAuthed.value ? (auth.user?.email ?? '') : t('chrome.account.label')))

async function signOut(): Promise<void> {
    await auth.logout()
    await router.replace('/auth/sign-in')
}
</script>

<template>
    <v-menu location="bottom end">
        <template #activator="{ props: activatorProps }">
            <v-btn
                icon
                :title="activatorLabel"
                :aria-label="activatorLabel"
                data-testid="user-menu"
                v-bind="activatorProps"
            >
                <!-- Authenticated → user's DefaultAvatar (initials or photo);
                     anonymous → the plain `user` icon, tinted with the
                     signature accent so the sign-in affordance is obvious
                     on the chromeless login / public surfaces. Same outer
                     v-btn, same 36 px hit-area in both states, so the
                     chrome geometry never hops on sign-in / sign-out. -->
                <span v-if="isAuthed" aria-hidden="true">
                    <DefaultAvatar :src="avatarUrl" :name="displayName" :size="36" />
                </span>
                <v-icon v-else icon="user" size="22" color="primary" aria-hidden="true" />
            </v-btn>
        </template>
        <v-list density="comfortable" data-testid="account-menu">
            <template v-if="isAuthed">
                <v-list-item
                    to="/account"
                    prepend-icon="user"
                    :title="t('account.menu.openAccount')"
                    data-testid="user-menu-account"
                />
                <v-list-item
                    prepend-icon="log-out"
                    :title="t('account.menu.signOut')"
                    data-testid="sign-out"
                    @click="signOut"
                />
            </template>
            <template v-else>
                <v-list-item
                    to="/auth/sign-in"
                    prepend-icon="log-in"
                    :title="t('chrome.account.login')"
                    data-testid="account-login"
                    color="primary"
                />
                <v-list-item
                    to="/auth/sign-in"
                    prepend-icon="user-plus"
                    :title="t('chrome.account.register')"
                    data-testid="account-register"
                />
            </template>
        </v-list>
    </v-menu>
</template>
