<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import FamilySwitcher from './FamilySwitcher.vue'
import LangSwitcher from './LangSwitcher.vue'

const { t } = useI18n()
const auth = useAuthStore()
const ui = useUiStore()
const router = useRouter()

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
                <v-btn icon="user" :title="auth.user?.email ?? ''" data-testid="user-menu" v-bind="activatorProps" />
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
