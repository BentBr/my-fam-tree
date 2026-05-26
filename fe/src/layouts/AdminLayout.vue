<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute } from 'vue-router'

import AppBar from '@/components/layout/AppBar.vue'

const { t } = useI18n()
const route = useRoute()

// Side-rail items. The `enabled` flag remains on the type so future
// surfaces (reminders, settings) can ship disabled until their view
// lands without producing 404s.
interface AdminNavItem {
    key: string
    to: string
    label: string
    enabled: boolean
}

const items = computed<AdminNavItem[]>(() => [
    { key: 'members', to: '/admin/members', label: t('admin.nav.members'), enabled: true },
    { key: 'invites', to: '/admin/invites', label: t('admin.nav.invites'), enabled: true },
    { key: 'audit', to: '/admin/audit', label: t('admin.nav.audit'), enabled: true },
])
</script>

<template>
    <AppBar />
    <v-main>
        <v-container fluid class="admin-shell">
            <aside class="rail" data-testid="admin-rail">
                <ul class="rail-list">
                    <li v-for="item in items" :key="item.key">
                        <RouterLink
                            v-if="item.enabled"
                            :to="item.to"
                            :class="{ active: route.path.startsWith(item.to) }"
                            :data-testid="`admin-rail-${item.key}`"
                        >
                            {{ item.label }}
                        </RouterLink>
                        <span
                            v-else
                            class="rail-disabled"
                            :data-testid="`admin-rail-${item.key}-disabled`"
                            :title="$t('common.readOnly')"
                        >
                            {{ item.label }}
                        </span>
                    </li>
                </ul>
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
    padding: 1rem 0.5rem;
    border-right: 1px solid rgba(0, 0, 0, 0.08);
}
.rail-list {
    list-style: none;
    padding: 0;
    margin: 0;
}
.rail-list li + li {
    margin-top: 0.25rem;
}
.rail-list a,
.rail-list .rail-disabled {
    display: block;
    padding: 0.5rem 0.75rem;
    border-radius: 0.5rem;
    color: rgb(var(--v-theme-on-surface));
    text-decoration: none;
}
.rail-list a.active {
    background: rgba(var(--v-theme-primary), 0.12);
    color: rgb(var(--v-theme-primary));
    font-weight: 600;
}
.rail-list .rail-disabled {
    color: rgba(var(--v-theme-on-surface), 0.4);
    cursor: not-allowed;
}
.content {
    min-width: 0;
    padding: 1rem 1.5rem;
}
</style>
