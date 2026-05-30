<script setup lang="ts">
/**
 * Unified application bar — used on every route, public and
 * authenticated. The layout is identical in both modes:
 *
 *   [nav toggle] [brand lockup]    [family switcher]    [theme] [lang] [account]
 *                                       ↑ auth only      ↑ always   ↑ always   ↑ flips by auth
 *
 * The brand lockup is a single shared atom (`BrandLogo`), the right-
 * side controls are three more atoms, all in `components/common/`.
 * Nothing here owns geometry beyond the v-app-bar wrapper — the atoms
 * own their own styling against the design tokens. That gives the
 * "no visual hop between public + authenticated" promise from the
 * plan.
 *
 * The hamburger / `nav-toggle` only renders when the sidebar exists
 * for the current route (authenticated views). On the public + auth-
 * gate views the toggle is hidden because there is no sidebar to
 * toggle.
 */
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import AccountControl from '@/components/common/AccountControl.vue'
import BrandLogo from '@/components/common/BrandLogo.vue'
import LanguageMenu from '@/components/common/LanguageMenu.vue'
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'

import FamilySwitcher from './FamilySwitcher.vue'

const auth = useAuthStore()
const ui = useUiStore()
const route = useRoute()

// The login / sign-up / consume / invite-accept flow uses a chromeless
// layout (no NavDrawer mounted): the hamburger + family switcher would
// reference things that don't exist. Hide them there. Every other
// layout (Main, Admin) carries the drawer — so the hamburger renders
// and the family switcher follows when the caller is signed in.
//
// Phase E will refine this to read `route.meta.sidebar` once every
// route opts in explicitly; for now `meta.layout` is the boundary
// that already exists and that AdminLayout / MainLayout / LoginLayout
// already agree on.
const isLoginLayout = computed(() => route.meta['layout'] === 'login')
const showSidebarToggle = computed(() => !isLoginLayout.value)
const showFamilySwitcher = computed(() => auth.status === 'authenticated' && !isLoginLayout.value)
</script>

<template>
    <!--
        `padding-inline` lives on the deep `.v-toolbar__content` slot
        (see <style> below) so the brand on the left and the controls
        on the right have breathing room from the viewport edges —
        matches the handoff's `clamp(14px, 3vw, 22px)` rule.
    -->
    <v-app-bar elevation="1" density="comfortable" data-testid="app-bar" class="app-bar app-bar--padded">
        <v-app-bar-nav-icon v-if="showSidebarToggle" icon="menu" data-testid="nav-toggle" @click="ui.toggleSidebar" />
        <BrandLogo to="/" size="md" />
        <v-spacer />
        <FamilySwitcher v-if="showFamilySwitcher" class="mr-2" />
        <ThemeToggle class="mr-1" />
        <LanguageMenu class="mr-1" />
        <AccountControl />
    </v-app-bar>
</template>

<style scoped>
/* `v-app-bar` ships with its own internal flex container; we hook the
   horizontal padding onto its deep `.v-toolbar__content` slot rather
   than the outer wrapper so Vuetify's own padding rules don't take
   precedence. Values mirror the design handoff. */
.app-bar--padded :deep(.v-toolbar__content) {
    padding-inline: clamp(14px, 3vw, 22px);
}
</style>
