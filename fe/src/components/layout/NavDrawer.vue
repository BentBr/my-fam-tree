<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDisplay } from 'vuetify'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useUiStore } from '@/stores/ui'

const { t } = useI18n()
const ui = useUiStore()
const family = useActiveFamilyStore()
const { smAndDown } = useDisplay()

// Two distinct layouts share one toggle flag (`ui.sidebarCollapsed`, driven by
// the app-bar hamburger) but read it differently per breakpoint:
//   - Desktop (sm+): the drawer is always present and `permanent`; the flag
//     switches it between full width and a compact icon `rail`. Unchanged.
//   - smAndDown (phones/small tablets): a permanent full-width drawer eats the
//     viewport, so we make it `temporary` (overlays content behind a scrim)
//     and hidden by default. The hamburger flips the flag to slide it in.
// `railMode` is desktop-only — a rail still costs ~56px and looks odd on a
// phone, so on small screens we never rail, we hide.
const railMode = computed(() => !smAndDown.value && ui.sidebarCollapsed)
const open = computed(() => (smAndDown.value ? ui.sidebarCollapsed : true))

// Vuetify emits `update:model-value` when the temporary drawer is dismissed by
// tapping the scrim or pressing Esc. Sync that back into the shared flag so the
// next hamburger tap can reopen it; without this the flag would stay `true`
// while the drawer is visually closed and the toggle would feel dead.
function onUpdateOpen(value: boolean): void {
    if (smAndDown.value && value !== ui.sidebarCollapsed) ui.toggleSidebar()
}

// On a phone the overlay drawer should get out of the way once the user picks a
// destination; on desktop the drawer is permanent so there is nothing to close.
function onItemClick(): void {
    if (smAndDown.value && ui.sidebarCollapsed) ui.toggleSidebar()
}

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
    ]
    if (isAdmin.value) {
        list.push({ to: '/admin/audit', title: t('nav.admin'), icon: 'shield', testId: 'nav-admin' })
    }
    return list
})
</script>

<template>
    <v-navigation-drawer
        :model-value="open"
        :rail="railMode"
        :permanent="!smAndDown"
        :temporary="smAndDown"
        data-testid="nav-drawer"
        @update:model-value="onUpdateOpen"
    >
        <v-list density="comfortable" nav>
            <v-list-item
                v-for="item in items"
                :key="item.to"
                :to="item.to"
                :prepend-icon="item.icon"
                :title="item.title"
                :data-testid="item.testId"
                color="primary"
                @click="onItemClick"
            />
        </v-list>

        <!-- Health is a low-traffic diagnostics page: demote it to a small,
             muted footnote in the drawer footer. Plain router-link, so simply
             rendering the nav never triggers a /health request. -->
        <template #append>
            <RouterLink to="/health" class="health-footnote" data-testid="nav-health-footer" :title="t('nav.health')">
                <v-icon icon="activity" size="x-small" />
                <span v-if="!railMode" class="ml-1">{{ t('nav.health') }}</span>
            </RouterLink>
        </template>
    </v-navigation-drawer>
</template>

<style scoped>
.health-footnote {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.5rem;
    font-size: 0.72rem;
    color: rgba(var(--v-theme-on-surface), 0.5);
    text-decoration: none;
}
.health-footnote:hover {
    color: rgba(var(--v-theme-on-surface), 0.8);
}
</style>
