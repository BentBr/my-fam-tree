<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import AppBar from '@/components/layout/AppBar.vue'
import NavDrawer from '@/components/layout/NavDrawer.vue'

const { t } = useI18n()

// Side-rail items. The `enabled` flag remains on the type so future
// surfaces (reminders, settings) can ship disabled until their view
// lands without producing 404s. Each item maps to a Lucide icon name
// (Vuetify resolves `prepend-icon` through the configured icon set) so
// the rail matches the look + feel of the main `NavDrawer`.
interface AdminNavItem {
    key: string
    to: string
    label: string
    icon: string
    enabled: boolean
}

const items = computed<AdminNavItem[]>(() => [
    { key: 'members', to: '/admin/members', label: t('admin.nav.members'), icon: 'users', enabled: true },
    { key: 'invites', to: '/admin/invites', label: t('admin.nav.invites'), icon: 'mail', enabled: true },
    { key: 'audit', to: '/admin/audit', label: t('admin.nav.audit'), icon: 'list', enabled: true },
])
</script>

<template>
    <AppBar />
    <NavDrawer />
    <v-main>
        <v-container fluid class="admin-shell">
            <aside class="rail" data-testid="admin-rail">
                <!-- Back-to-tree affordance. The main nav also surfaces
                     /tree, but a dedicated rail link is the most obvious
                     escape hatch when the user is deep in /admin/audit. -->
                <v-list density="compact" nav class="rail-list">
                    <v-list-item
                        to="/tree"
                        prepend-icon="arrow-left"
                        :title="t('admin.nav.back')"
                        data-testid="admin-rail-back"
                        color="primary"
                    />
                    <v-divider class="my-1" />
                    <template v-for="item in items" :key="item.key">
                        <v-list-item
                            v-if="item.enabled"
                            :to="item.to"
                            :prepend-icon="item.icon"
                            :title="item.label"
                            :data-testid="`admin-rail-${item.key}`"
                            color="primary"
                        />
                        <v-list-item
                            v-else
                            :prepend-icon="item.icon"
                            :title="item.label"
                            :data-testid="`admin-rail-${item.key}-disabled`"
                            disabled
                        />
                    </template>
                </v-list>
            </aside>
            <main class="content">
                <router-view v-slot="{ Component, route: r }">
                    <transition name="fade" mode="out-in" appear>
                        <component :is="Component" :key="r.path" />
                    </transition>
                </router-view>
            </main>
        </v-container>
    </v-main>
</template>

<style scoped>
.admin-shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 1rem;
    min-height: 100%;
}
.rail {
    padding: 0.5rem 0.25rem;
    border-right: 1px solid rgba(0, 0, 0, 0.08);
}
.rail-list {
    background: transparent;
}
.content {
    min-width: 0;
    padding: 1rem 1.5rem;
}
</style>
