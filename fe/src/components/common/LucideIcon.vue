<script setup lang="ts">
import * as icons from '@lucide/vue'
import { computed, type Component } from 'vue'

const props = withDefaults(
    defineProps<{
        name: string
        size?: number | string
        color?: string
        strokeWidth?: number
    }>(),
    {
        size: 20,
        color: '',
        strokeWidth: 1.75,
    },
)

const iconComp = computed<Component | null>(() => {
    // lucide exports PascalCase component names — convert "user-plus" → "UserPlus".
    const pascal = props.name
        .split('-')
        .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
        .join('')
    const all = icons as unknown as Record<string, Component>
    return all[pascal] ?? null
})

// Lucide ships only a `+`-shaped `cross` (medical); for memorials we want a
// Christian Latin cross with a longer vertical bar. Rendered inline so we
// don't have to ship another icon package or special-case at every call site
// — `<LucideIcon name="latin-cross" />` just works.
const isLatinCross = computed(() => props.name === 'latin-cross')
const sizePx = computed(() => (typeof props.size === 'number' ? props.size : Number(props.size)))
const strokeBinding = computed(() => (props.color === '' ? 'currentColor' : props.color))

const colorBinding = computed(() => (props.color === '' ? {} : { color: props.color }))
</script>

<template>
    <svg
        v-if="isLatinCross"
        :width="sizePx"
        :height="sizePx"
        viewBox="0 0 24 24"
        fill="none"
        :stroke="strokeBinding"
        :stroke-width="props.strokeWidth"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
    >
        <!-- Vertical bar slightly taller, crossbar at upper third — the
             canonical Latin / Christian cross silhouette. -->
        <line x1="12" y1="3" x2="12" y2="21" />
        <line x1="7" y1="9" x2="17" y2="9" />
    </svg>
    <component
        :is="iconComp"
        v-else-if="iconComp"
        :size="props.size"
        :stroke-width="props.strokeWidth"
        v-bind="colorBinding"
    />
</template>
