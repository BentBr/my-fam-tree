<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import FamilySwitcher from './FamilySwitcher.vue'
import LangSwitcher from './LangSwitcher.vue'

const { t } = useI18n()
const auth = useAuthStore()
const ui = useUiStore()
</script>

<template>
    <v-app-bar elevation="1" density="comfortable" data-testid="app-bar">
        <v-app-bar-nav-icon icon="menu" @click="ui.toggleSidebar" />
        <v-app-bar-title>{{ t('app.title') }}</v-app-bar-title>
        <v-spacer />
        <FamilySwitcher class="mr-2" />
        <LangSwitcher class="mr-2" />
        <v-btn
            v-if="auth.status === 'authenticated'"
            icon="log-out"
            :title="t('auth.signOut')"
            data-testid="sign-out"
            @click="auth.logout"
        />
    </v-app-bar>
</template>
