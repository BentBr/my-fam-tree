<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import LoginLayout from '@/layouts/LoginLayout.vue'
import MainLayout from '@/layouts/MainLayout.vue'

const route = useRoute()
const layout = computed<'login' | 'main'>(() =>
    (route.meta.layout as 'login' | 'main' | undefined) ?? 'main',
)
</script>

<template>
    <v-app>
        <LoginLayout v-if="layout === 'login'" />
        <MainLayout v-else />
    </v-app>
</template>

<style>
/* scrollbar-gutter keeps the page from shifting during fade transitions */
html, body { scrollbar-gutter: stable; margin: 0; min-height: 100vh; }
</style>
