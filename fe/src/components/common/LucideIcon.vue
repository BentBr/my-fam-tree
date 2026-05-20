<script setup lang="ts">
import * as icons from 'lucide-vue-next'
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

const colorBinding = computed(() => (props.color === '' ? {} : { color: props.color }))
</script>

<template>
    <component
        :is="iconComp"
        v-if="iconComp"
        :size="props.size"
        :stroke-width="props.strokeWidth"
        v-bind="colorBinding"
    />
</template>
