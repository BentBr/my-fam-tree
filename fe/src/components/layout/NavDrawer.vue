<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDisplay } from 'vuetify'

import { useUiStore } from '@/stores/ui'

const { t } = useI18n()
const ui = useUiStore()
const { mobile } = useDisplay()
const railMode = computed(() => !mobile.value && ui.sidebarCollapsed)
const open = computed(() => !mobile.value || !ui.sidebarCollapsed)

const items = computed(() => [
    { to: '/tree', title: t('nav.tree'), icon: 'network' },
    { to: '/reminders/history', title: t('nav.reminders'), icon: 'bell' },
    { to: '/health', title: t('nav.health'), icon: 'activity' },
])
</script>

<template>
    <v-navigation-drawer :model-value="open" :rail="railMode" permanent data-testid="nav-drawer">
        <v-list density="comfortable" nav>
            <v-list-item
                v-for="item in items"
                :key="item.to"
                :to="item.to"
                :prepend-icon="item.icon"
                :title="item.title"
                active-color="primary"
            />
        </v-list>
    </v-navigation-drawer>
</template>
