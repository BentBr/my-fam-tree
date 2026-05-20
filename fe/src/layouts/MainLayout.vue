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
                    <component :is="Component" :key="route.fullPath" />
                </transition>
            </router-view>
        </v-container>
    </v-main>
</template>
