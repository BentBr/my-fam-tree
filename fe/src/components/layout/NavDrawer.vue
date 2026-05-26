<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDisplay } from 'vuetify'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useUiStore } from '@/stores/ui'

const { t } = useI18n()
const ui = useUiStore()
const family = useActiveFamilyStore()
const { mobile } = useDisplay()
const railMode = computed(() => !mobile.value && ui.sidebarCollapsed)
const open = computed(() => !mobile.value || !ui.sidebarCollapsed)

// `/reminders/history` is added by Phase 4b. Until the route + view land,
// listing it here triggers a vue-router "No match found" warning on every
// navigation; keep the nav truthful to what actually exists.
// The admin entry only renders when the active membership grants admin
// or owner — matches the router guard on `/admin/*`.
interface NavItem {
    to: string
    title: string
    icon: string
    testId?: string
}

const isAdmin = computed(() => {
    const role = family.activeFamily?.role ?? null
    return role === 'admin' || role === 'owner'
})

const items = computed<NavItem[]>(() => {
    const list: NavItem[] = [
        { to: '/tree', title: t('nav.tree'), icon: 'network' },
        { to: '/upcoming', title: t('nav.upcoming'), icon: 'calendar-clock' },
        { to: '/health', title: t('nav.health'), icon: 'activity' },
    ]
    if (isAdmin.value) {
        list.push({ to: '/admin/audit', title: t('nav.admin'), icon: 'shield', testId: 'nav-admin' })
    }
    return list
})
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
                :data-testid="item.testId"
                color="primary"
            />
        </v-list>
    </v-navigation-drawer>
</template>
