<script setup lang="ts">
/**
 * Sign-in / magic-link / invite-accept layout.
 *
 * Same `AppBar` as the rest of the app — the brand block + theme
 * toggle + language menu live at the top of every screen, including
 * the unauthenticated ones. The auth-state-aware `AccountControl`
 * renders the Login/Register CTAs when the user isn't signed in, so
 * the right-side cluster stays full even here.
 *
 * The `<v-main>` body itself keeps the radial-gradient backdrop +
 * 480 px-wide centred card that the sign-in / consume / invite views
 * expect — that's still the right look for a focused single-form
 * surface.
 */
import AppBar from '@/components/layout/AppBar.vue'
</script>

<template>
    <AppBar />
    <v-main class="login-main">
        <v-container class="d-flex align-center justify-center" fluid>
            <div class="login-frame">
                <div class="fade-router-view">
                    <router-view v-slot="{ Component, route }">
                        <transition name="fade" mode="out-in" appear>
                            <component :is="Component" :key="route.fullPath" />
                        </transition>
                    </router-view>
                </div>
            </div>
        </v-container>
    </v-main>
</template>

<style scoped>
.login-main {
    min-height: 100vh;
    background:
        radial-gradient(1200px 600px at 10% -10%, rgb(var(--v-theme-primary) / 0.12), transparent 60%),
        radial-gradient(900px 500px at 90% 110%, rgb(var(--v-theme-secondary) / 0.1), transparent 60%),
        rgb(var(--v-theme-background));
}
.login-frame {
    width: 100%;
    max-width: 480px;
}
</style>
