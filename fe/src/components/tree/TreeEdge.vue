<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
    kind: 'parent' | 'partner'
    ax: number
    ay: number
    bx: number
    by: number
}>()

// Parent edges draw a vertical cubic Bezier from child top up to parent bottom
// so multi-generation stacks read as smooth lineages. Partner edges are flat
// horizontal lines so the heart glyph sits naturally on the midpoint.
const path = computed(() => {
    if (props.kind === 'parent') {
        const midY = (props.ay + props.by) / 2
        return `M ${props.ax} ${props.ay} C ${props.ax} ${midY}, ${props.bx} ${midY}, ${props.bx} ${props.by}`
    }
    return `M ${props.ax} ${props.ay} L ${props.bx} ${props.by}`
})

const midX = computed(() => (props.ax + props.bx) / 2)
const midY = computed(() => (props.ay + props.by) / 2)
</script>

<template>
    <g aria-hidden="true" :class="['edge-group', kind]" :data-testid="`tree-edge-${kind}`">
        <path :d="path" class="edge" fill="none" />
        <!-- Heart glyph at midpoint for partner edges; pure SVG scales with zoom. -->
        <g v-if="kind === 'partner'" :transform="`translate(${midX - 6} ${midY - 6})`" class="heart">
            <path d="M6 11 L1 6 a3 3 0 0 1 5 -2 a3 3 0 0 1 5 2 z" />
        </g>
    </g>
</template>

<style scoped>
.edge-group .edge {
    fill: none;
}
.edge-group.parent .edge {
    /* NB: avoid `rgb(var(--v-theme-on-surface) / α)` — Vuetify's theme tokens
     * are emitted as `R, G, B` (comma-separated), and the CSS slash-alpha
     * syntax requires SPACE-separated channels. The mixed form silently
     * resolves to `stroke: none` and the edges vanish. A concrete neutral
     * grey is fine for the parent-link visual treatment. */
    stroke: #94a3b8;
    stroke-width: 2;
    stroke-linecap: round;
}
.edge-group.partner .edge {
    stroke: rgb(var(--v-theme-secondary));
    stroke-width: 2;
    stroke-dasharray: 6 4;
}
.heart path {
    fill: rgb(var(--v-theme-secondary));
    opacity: 0.85;
}
</style>
