<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import ToastContainer from '@/components/common/ToastContainer.vue'
import { useThemeMode } from '@/composables/useThemeMode'
import AdminLayout from '@/layouts/AdminLayout.vue'
import LoginLayout from '@/layouts/LoginLayout.vue'
import MainLayout from '@/layouts/MainLayout.vue'

const route = useRoute()
const layout = computed<'login' | 'main' | 'admin'>(
    () => (route.meta.layout as 'login' | 'main' | 'admin' | undefined) ?? 'main',
)

// Single owner of the `<html data-theme>` attribute + Vuetify theme
// sync; reads the persisted ThemeMode from `useUiStore`. Mounted at
// the root so the side effect is process-wide.
useThemeMode()
</script>

<template>
    <v-app>
        <LoginLayout v-if="layout === 'login'" />
        <AdminLayout v-else-if="layout === 'admin'" />
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
