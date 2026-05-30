<script setup lang="ts">
/**
 * Public-facing layout — the home page, imprint, and data policy.
 *
 * Mounts the SAME `AppBar` as the authenticated layouts (the brand
 * block, theme toggle, language menu, and account control all read
 * the same atoms; only the family switcher is suppressed because no
 * family scope exists here).
 *
 * The footer below `<router-view>` carries the legal-page links —
 * `/imprint` and `/data-policy` only appear here (they're never in
 * the AppBar nav).
 */
import AppBar from '@/components/layout/AppBar.vue'
import PublicFooter from '@/components/layout/PublicFooter.vue'
</script>

<template>
    <AppBar />
    <v-main class="public-main">
        <div class="public-shell">
            <router-view v-slot="{ Component, route }">
                <transition name="fade" mode="out-in" appear>
                    <component :is="Component" :key="route.fullPath" />
                </transition>
            </router-view>
        </div>
        <PublicFooter />
    </v-main>
</template>

<style scoped>
.public-main {
    min-height: 100vh;
    /* Soft warm-radial backdrop, exactly the same recipe LoginLayout
       uses for the sign-in card — keeps the brand feel consistent
       across the chromeless surfaces. The radial gradients tint the
       background with the accent + secondary roles so the orange
       signature peeks through without overwhelming. */
    background:
        radial-gradient(1200px 600px at 10% -10%, rgb(var(--v-theme-primary) / 0.12), transparent 60%),
        radial-gradient(900px 500px at 90% 110%, rgb(var(--v-theme-secondary) / 0.1), transparent 60%),
        rgb(var(--v-theme-background));
}
.public-shell {
    /* 1200 px reading width clamp matches MainLayout's authenticated
       gutter — keeps paragraphs comfortable on ultrawides. Mobile
       falls back to full width with side padding inside each view. */
    width: 100%;
    max-width: 1200px;
    margin-inline: auto;
    padding-inline: clamp(16px, 4vw, 32px);
    padding-block: clamp(24px, 5vw, 56px);
}
</style>
