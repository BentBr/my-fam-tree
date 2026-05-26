<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import AppBar from '@/components/layout/AppBar.vue'
import NavDrawer from '@/components/layout/NavDrawer.vue'

const route = useRoute()

// Pages that need the full canvas width opt out of the 1200px clamp.
// `/tree` is the only one for now — the SVG layout fills horizontal
// space and the cards would feel cramped if we squeezed them into the
// reading-text gutter. New full-width surfaces (a future kanban, a
// timeline) get listed here too. Auth views never reach this layout,
// so they don't need an entry.
const WIDE_ROUTES = new Set(['/tree'])
const isWide = computed(() => WIDE_ROUTES.has(route.path))
</script>

<template>
    <AppBar />
    <NavDrawer />
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
/* Text-heavy pages clamp to 1200px on md+ breakpoints so paragraphs and
 * forms stay within a comfortable reading width on ultrawide displays.
 * Pages that need the full canvas (the tree SVG) opt out via the
 * `--wide` modifier set in script-setup above. The base padding is
 * preserved at all widths — v-container's own padding handles the
 * narrow side. */
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
