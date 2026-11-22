<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import ToastContainer from '@/components/common/ToastContainer.vue'
import { useRouteMeta } from '@/composables/useRouteMeta'
import { useThemeMode } from '@/composables/useThemeMode'
import LoginLayout from '@/layouts/LoginLayout.vue'
import MainLayout from '@/layouts/MainLayout.vue'
import PublicLayout from '@/layouts/PublicLayout.vue'

type Layout = 'login' | 'main' | 'public'

const route = useRoute()
const layout = computed<Layout>(() => route.meta.layout ?? 'main')

// Single owner of the `<html data-theme>` attribute + Vuetify theme
// sync; reads the persisted ThemeMode from `useUiStore`. Mounted at
// the root so the side effect is process-wide.
useThemeMode()

// Single owner of the document head — resolves the active route's SEO
// descriptor (title + description + OG/Twitter + canonical/hreflang)
// through the shared `usePageMeta` composable. Mounted once here so
// every route (public + authenticated) gets metadata with no per-view
// wiring; the head re-renders on navigation and on locale switch.
useRouteMeta()
</script>

<template>
    <v-app>
        <PublicLayout v-if="layout === 'public'" />
        <LoginLayout v-else-if="layout === 'login'" />
        <MainLayout v-else />
        <!-- ToastContainer lives outside the layout switch so toasts persist
             across login/main transitions. -->
        <ToastContainer />
    </v-app>
</template>

<style>
/* scrollbar-gutter keeps the page from shifting during fade transitions */
html,
body {
    scrollbar-gutter: stable;
    margin: 0;
    min-height: 100vh;
}
</style>
