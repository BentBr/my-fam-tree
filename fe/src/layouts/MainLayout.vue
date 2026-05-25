<script setup lang="ts">
import AppBar from '@/components/layout/AppBar.vue'
import NavDrawer from '@/components/layout/NavDrawer.vue'
</script>

<template>
    <AppBar />
    <NavDrawer />
    <v-main>
        <v-container fluid class="fade-router-view">
            <router-view v-slot="{ Component, route }">
                <transition name="fade" mode="out-in" appear>
                    <!-- Key on `path`, not `fullPath`. Including the query
                         string here would unmount + remount the route
                         component on every `router.replace({ query })`
                         call — which is exactly what TreeView does to
                         strip a one-shot `?center=` param after capturing
                         it. Path-only is the standard pattern. -->
                    <component :is="Component" :key="route.path" />
                </transition>
            </router-view>
        </v-container>
    </v-main>
</template>
