<script setup lang="ts">
/**
 * Unified application sidebar.
 *
 * One <v-navigation-drawer> shell with one density rule; the *items*
 * inside switch based on the route's `meta.sidebar`:
 *
 *   'main'  → Tree, Upcoming, conditional Admin entry.
 *   'admin' → Back-to-tree, Members, Pending invites, Audit log.
 *   'none'  → drawer hidden (public + sign-in surfaces).
 *
 * The first row is always the brand block (icon-only when railed,
 * full lockup when expanded). Item geometry — height, density,
 * icon-to-text gutter, padding — is identical between variants so
 * switching `/tree` → `/admin/members` doesn't shift the column or
 * the row positions.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { useDisplay } from 'vuetify'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useUiStore } from '@/stores/ui'

const { t } = useI18n()
const ui = useUiStore()
const family = useActiveFamilyStore()
const route = useRoute()
const { smAndDown } = useDisplay()

interface SidebarItem {
    to: string
    title: string
    icon: string
    testId?: string
}

const variant = computed(() => route.meta.sidebar ?? 'none')

const isAdmin = computed(() => {
    const role = family.activeFamily?.role ?? null
    return role === 'admin' || role === 'owner'
})

const mainItems = computed<SidebarItem[]>(() => {
    const list: SidebarItem[] = [
        { to: '/tree', title: t('nav.tree'), icon: 'network' },
        { to: '/upcoming', title: t('nav.upcoming'), icon: 'calendar-clock' },
    ]
    if (isAdmin.value) {
        list.push({ to: '/admin/audit', title: t('nav.admin'), icon: 'shield', testId: 'nav-admin' })
    }
    return list
})

const adminItems = computed<SidebarItem[]>(() => [
    { to: '/tree', title: t('admin.nav.back'), icon: 'arrow-left', testId: 'admin-rail-back' },
    { to: '/admin/family', title: t('admin.nav.family'), icon: 'home', testId: 'admin-rail-family' },
    { to: '/admin/members', title: t('admin.nav.members'), icon: 'users', testId: 'admin-rail-members' },
    { to: '/admin/invites', title: t('admin.nav.invites'), icon: 'mail', testId: 'admin-rail-invites' },
    { to: '/admin/audit', title: t('admin.nav.audit'), icon: 'list', testId: 'admin-rail-audit' },
])

const items = computed<SidebarItem[]>(() => (variant.value === 'admin' ? adminItems.value : mainItems.value))

// Two distinct layouts share one toggle flag (`ui.sidebarCollapsed`, driven by
// the AppBar hamburger) but read it differently per breakpoint:
//   - Desktop (sm+): the drawer is always present and `permanent`; the flag
//     switches it between full width and a compact icon `rail`.
//   - smAndDown (phones / small tablets): a permanent full-width drawer eats
//     the viewport, so we make it `temporary` (overlays content behind a
//     scrim) and hidden by default. The hamburger flips the flag to slide
//     it in.
// `railMode` is desktop-only — a rail still costs ~56 px and looks odd on a
// phone, so on small screens we never rail, we hide.
const railMode = computed(() => !smAndDown.value && ui.sidebarCollapsed)
const open = computed(() => (smAndDown.value ? ui.sidebarCollapsed : true))

// Vuetify emits `update:model-value` when the temporary drawer is dismissed
// by tapping the scrim or pressing Esc. Sync that back into the shared flag
// so the next hamburger tap can reopen it; without this the flag would stay
// `true` while the drawer is visually closed and the toggle would feel dead.
function onUpdateOpen(value: boolean): void {
    if (smAndDown.value && value !== ui.sidebarCollapsed) ui.toggleSidebar()
}

// On a phone the overlay drawer should get out of the way once the user picks
// a destination; on desktop the drawer is permanent so there is nothing to
// close.
function onItemClick(): void {
    if (smAndDown.value && ui.sidebarCollapsed) ui.toggleSidebar()
}
</script>

<template>
    <v-navigation-drawer
        v-if="variant !== 'none'"
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

        <template #append>
            <RouterLink to="/health" class="health-footnote" data-testid="nav-health-footer" :title="t('nav.health')">
                <v-icon icon="activity" size="x-small" />
                <span v-if="!railMode" class="ml-1">{{ t('nav.health') }}</span>
            </RouterLink>
        </template>
    </v-navigation-drawer>
</template>

<style scoped>
.app-sidebar__brand {
    /* Same horizontal gutter as the list items below so the icon
       aligns column-wise with each row's prepend-icon. */
    padding: 14px 16px 6px;
}
.app-sidebar__brand--rail {
    padding: 12px 8px 6px;
    display: flex;
    justify-content: center;
}

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
