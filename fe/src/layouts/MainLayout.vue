<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import AppBar from '@/components/layout/AppBar.vue'
import AppSidebar from '@/components/layout/AppSidebar.vue'

const route = useRoute()

// Pages that need the full canvas width opt out of the 1200 px reading
// clamp. The tree SVG fills its column; the admin tables read better
// without the gutter. New full-width surfaces add their path here OR
// flip their route's `meta.sidebar` to `'admin'` (admin-sidebar routes
// are already considered wide).
const WIDE_ROUTES = new Set(['/tree'])
const isWide = computed(() => WIDE_ROUTES.has(route.path) || route.meta.sidebar === 'admin')
</script>

<template>
    <AppBar />
    <AppSidebar />
    <v-main>
        <v-container fluid class="fade-router-view page-container" :class="{ 'page-container--wide': isWide }">
            <router-view v-slot="{ Component, route: r }">
                <transition name="fade" mode="out-in" appear>
                    <!-- Key on `path`, not `fullPath`. Including the query
                         string here would unmount + remount the route
                         component on every `router.replace({ query })`
                         call — which is exactly what TreeView does to
                         strip a one-shot `?center=` param after capturing
                         it. Path-only is the standard pattern. -->
                    <component :is="Component" :key="r.path" />
                </transition>
            </router-view>
        </v-container>
    </v-main>
</template>

<style scoped>
/* Text-heavy pages clamp to 1200 px on md+ breakpoints so paragraphs
 * and forms stay within a comfortable reading width on ultrawide
 * displays. Pages that need the full canvas (the tree SVG, admin
 * tables) opt out via the `--wide` modifier set in script-setup
 * above. The base padding is preserved at all widths — v-container's
 * own padding handles the narrow side. */
.page-container {
    width: 100%;
}
@media (min-width: 960px) {
    .page-container:not(.page-container--wide) {
        max-width: 1200px;
        margin-left: auto;
        margin-right: auto;
    }
}
</style>
