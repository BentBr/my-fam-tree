<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useMe } from '@/api/hooks/users'
import DefaultAvatar from '@/components/common/DefaultAvatar.vue'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import FamilySwitcher from './FamilySwitcher.vue'
import LangSwitcher from './LangSwitcher.vue'

const { t } = useI18n()
const auth = useAuthStore()
const ui = useUiStore()
const router = useRouter()

// Profile fetch — gated on authenticated state so we don't hammer
// /users/me on every render of the splash / sign-in page where the
// store is still 'unauthenticated'.
const me = useMe()
const avatarUrl = computed(() => me.data.value?.avatar_url ?? null)
const displayName = computed(() => me.data.value?.display_name ?? auth.user?.email ?? '')

// `auth.logout()` clears the store but does not navigate. We always send the
// user to /auth/sign-in afterwards so the FE never leaves them on a now-empty
// authenticated page where the router guard would re-bounce on the next nav.
async function signOut(): Promise<void> {
    await auth.logout()
    await router.replace('/auth/sign-in')
}
</script>

<template>
    <v-app-bar elevation="1" density="comfortable" data-testid="app-bar">
        <v-app-bar-nav-icon icon="menu" data-testid="nav-toggle" @click="ui.toggleSidebar" />
        <v-app-bar-title>{{ t('app.title') }}</v-app-bar-title>
        <v-spacer />
        <FamilySwitcher class="mr-2" />
        <LangSwitcher class="mr-2" />
        <v-menu v-if="auth.status === 'authenticated'" location="bottom end">
            <template #activator="{ props: activatorProps }">
                <v-btn icon :title="auth.user?.email ?? ''" data-testid="user-menu" v-bind="activatorProps">
                    <DefaultAvatar :src="avatarUrl" :name="displayName" :size="36" />
                </v-btn>
            </template>
            <v-list density="comfortable">
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
            </v-list>
        </v-menu>
    </v-app-bar>
</template>
