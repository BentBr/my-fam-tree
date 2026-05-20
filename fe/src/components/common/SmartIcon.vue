<script setup lang="ts">
import { computed } from 'vue'

import LucideIcon from './LucideIcon.vue'

/**
 * Vuetify hands this component an `icon` prop whenever a built-in component
 * (e.g. `<v-btn icon="user">`) needs to render an icon. We forward the string
 * form to LucideIcon and ignore complex icon shapes (array / component) — the
 * built-in components only ever feed us strings via the default icon set.
 *
 * Vuetify's IconComponent contract requires `tag` to be present and `icon` to
 * be optional; we expose a permissive subset. The component is cast at the
 * registration site in `main.ts` because Vuetify's `JSXComponent` constructor
 * shape is narrower than Vue's `defineComponent` output.
 */
const props = defineProps<{
    icon?: unknown
    tag?: string
    disabled?: boolean
}>()

const name = computed(() => {
    const raw = typeof props.icon === 'string' ? props.icon : ''
    return raw.replace(/^mdi-/, '').replace(/^lucide-/, '')
})
</script>

<template>
    <LucideIcon v-if="name" :name="name" />
</template>
