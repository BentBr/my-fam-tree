<script setup lang="ts">
/**
 * Brand lockup — the sloth icon + "my-family" wordmark + "by Slothlike"
 * subline. Single source of the chrome's home identity; used by the
 * AppBar on every route (public + authenticated) and by the sidebar
 * header. The exact same DOM in both, so geometry never drifts between
 * surfaces.
 *
 * Sizes:
 *   sm — 24 × 24 icon, no text (rail-mode of the sidebar).
 *   md — 36 × 36 icon, wordmark + subline (AppBar default).
 *   lg — 48 × 48 icon, larger wordmark (hero / splash spots).
 *
 * The component is decorative-by-default. Pass `to` to make it act as
 * a router-link (used by the AppBar so clicking it returns home).
 */
import { computed } from 'vue'
import { RouterLink } from 'vue-router'

// Importing the brand raster lets Vite rewrite the resolved URL with a
// content hash (`/assets/sloth-128-[hash].webp`) at build time. A
// refreshed asset lands at a NEW URL so browsers + CDNs serve it
// without any cache-control bypass — the symptom the user hit when
// the updated sloth-family.png stayed cached in prod for a day.
import sloth128 from '@/assets/brand/sloth-128.webp'
import sloth256 from '@/assets/brand/sloth-256.webp'
import sloth512 from '@/assets/brand/sloth-512.webp'

const slothSrcset = `${sloth128} 128w, ${sloth256} 256w, ${sloth512} 512w`

const props = withDefaults(
    defineProps<{
        /** Visual scale. */
        size?: 'sm' | 'md' | 'lg'
        /** Router path. Omit (or pass `null`) to render plain markup. */
        to?: string | null
    }>(),
    { size: 'md', to: null },
)

const iconPx = computed(() => (props.size === 'lg' ? 48 : props.size === 'md' ? 36 : 24))
const wordPx = computed(() => (props.size === 'lg' ? 22 : 18))
const subPx = computed(() => (props.size === 'lg' ? 11.5 : 10.5))
const showText = computed(() => props.size !== 'sm')
const linked = computed(() => props.to !== null)
</script>

<template>
    <component
        :is="linked ? RouterLink : 'div'"
        :to="to ?? undefined"
        class="brand-lockup"
        :class="{ 'brand-lockup--linked': linked }"
        data-testid="brand-logo"
    >
        <img
            class="brand-lockup__icon"
            :width="iconPx"
            :height="iconPx"
            :src="sloth128"
            :srcset="slothSrcset"
            sizes="48px"
            alt=""
        />
        <span v-if="showText" class="brand-lockup__text">
            <span class="brand-lockup__word display" :style="{ fontSize: `${wordPx}px` }">My Family Tree</span>
            <span class="brand-lockup__sub" :style="{ fontSize: `${subPx}px` }">by Slothlike</span>
        </span>
    </component>
</template>

<style scoped>
.brand-lockup {
    display: inline-flex;
    align-items: center;
    gap: 11px;
    user-select: none;
    text-decoration: none;
    color: inherit;
    flex-shrink: 0;
}
.brand-lockup--linked {
    cursor: pointer;
}
.brand-lockup__icon {
    display: block;
    flex-shrink: 0;
    border-radius: var(--r-sm);
    filter: drop-shadow(var(--shadow-sm));
}
.brand-lockup__text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    line-height: 1.05;
    white-space: nowrap;
}
.brand-lockup__word {
    font-weight: 700;
    color: var(--text);
    letter-spacing: -0.01em;
}
.brand-lockup__sub {
    font-weight: 600;
    color: var(--text-3);
    line-height: 1;
}
</style>
