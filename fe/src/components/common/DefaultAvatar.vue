<script setup lang="ts">
/**
 * Avatar renderer with photo + initials fallback.
 *
 * Always renders a `v-avatar`. When `src` is a non-empty string we show the
 * image; otherwise we show up to two letters from `name` over a colour
 * derived from a stable hash of `name` so the same person always gets the
 * same colour across reloads + devices.
 *
 * The component takes no opinion on size — pass through any `size` the
 * parent wants (Vuetify accepts numbers or t-shirt sizes). Default is `40`
 * to match Vuetify's `v-avatar` default.
 */
import { computed } from 'vue'

const props = withDefaults(
    defineProps<{
        /** Presigned URL or null/undefined when the person has no photo. */
        src?: string | null
        /** Display name used for initials + colour derivation. */
        name?: string
        /** Vuetify size prop — number, t-shirt string, or px string. */
        size?: number | string
    }>(),
    { src: null, name: '', size: 40 },
)

/**
 * Cheap deterministic hash so identical names land on identical colours
 * across sessions. Not crypto — just spread bytes through 32-bit space.
 */
function djb2(str: string): number {
    let h = 5381
    for (const ch of str) {
        h = ((h << 5) + h) ^ ch.charCodeAt(0)
    }
    return h >>> 0
}

/**
 * Curated palette: avoid yellow (low contrast on white text) and pure
 * red (alarming on a person card). HSL-ish bases with the same lightness
 * so the foreground text contrast is uniform.
 */
const PALETTE = [
    '#3949ab', // indigo
    '#1e88e5', // blue
    '#039be5', // light blue
    '#00897b', // teal
    '#43a047', // green
    '#7cb342', // light green
    '#fb8c00', // orange
    '#f4511e', // deep orange
    '#6d4c41', // brown
    '#5e35b1', // deep purple
    '#8e24aa', // purple
    '#d81b60', // pink
] as const

const initials = computed(() => {
    const trimmed = props.name.trim()
    if (trimmed === '') return '?'
    // Take the first letter of up to two whitespace-separated tokens.
    const parts = trimmed.split(/\s+/).slice(0, 2)
    return parts
        .map((p) => (p.length > 0 ? (p[0] ?? '').toUpperCase() : ''))
        .filter((c) => c !== '')
        .join('')
})

const bg = computed(() => {
    const key = props.name.trim() === '' ? '?' : props.name.trim()
    const idx = djb2(key) % PALETTE.length
    return PALETTE[idx] ?? PALETTE[0]
})
</script>

<template>
    <v-avatar
        :size="size"
        :color="src !== null && src !== undefined && src !== '' ? undefined : bg"
        data-testid="default-avatar"
    >
        <v-img v-if="src !== null && src !== undefined && src !== ''" :src="src" cover :alt="name" />
        <span
            v-else
            class="text-white font-weight-medium"
            :style="{ fontSize: 'calc(var(--v-avatar-height, 40px) * 0.4)' }"
        >
            {{ initials }}
        </span>
    </v-avatar>
</template>
