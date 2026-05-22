<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
    kind: 'parent' | 'partner'
    ax: number
    ay: number
    bx: number
    by: number
    /** Highlight this edge as part of the hover focus set — the canvas has
     * a hovered node and this edge connects it to one of its direct
     * relations. Full opacity + thicker stroke; drops dashes so partner
     * edges read crisply rather than fading with alpha. */
    isHighlighted?: boolean
    /** Edge is unrelated to the current hover focus; fade to 0.4 so the
     * highlighted subset reads cleanly. */
    isDimmed?: boolean
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
    <g
        aria-hidden="true"
        :class="['edge-group', kind, { highlighted: props.isHighlighted === true, dimmed: props.isDimmed === true }]"
        :data-testid="`tree-edge-${kind}`"
    >
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

/* When a node is hovered the connecting edges to its direct relations
 * get this treatment: thicker stroke, full opacity, no dashes — partner
 * edges otherwise read as a faint dashed line which is too low-contrast
 * to track as a "this is the relationship" cue. */
.edge-group.highlighted .edge {
    stroke-width: 3;
    stroke-dasharray: none;
    opacity: 1;
}
.edge-group.highlighted.partner .edge {
    /* Keep the warm secondary fill but drop the dash so the line connects
     * the hovered partner pair without visual chopping. */
    stroke: rgb(var(--v-theme-secondary));
}
.edge-group.highlighted .heart path {
    opacity: 1;
}

/* Unrelated edges fade so the highlighted ones pop. Same opacity floor
 * as TreeNode.dimmed so the visual treatment matches. */
.edge-group.dimmed {
    opacity: 0.4;
    transition: opacity 150ms ease-in-out;
}
</style>
