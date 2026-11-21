<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { NODE_W } from './layout'

const props = defineProps<{
    nodeId: string
    filled: boolean
}>()

const emit = defineEmits<{
    (e: 'toggle', id: string, next: boolean): void
}>()

const { t } = useI18n()

/**
 * Click on the star: flip the favourite mark. `stopPropagation` is
 * critical — without it the click bubbles to the surrounding card `<g>`
 * and also fires `select`, which would open the drawer every time the
 * user stars a card.
 */
function onStarClick(evt: Event): void {
    evt.stopPropagation()
    emit('toggle', props.nodeId, !props.filled)
}

const favouriteTooltip = computed(() =>
    t(props.filled ? 'person.favourite.unmarkTooltip' : 'person.favourite.markTooltip'),
)

// Star geometry. We hand-build the SVG `points` attribute for a 5-point
// star so the icon stays scoped to the tree node (no extra lucide icon, no
// runtime path lookups). The star sits in the top-right of the card; we
// centre at (cx, cy) with outer radius `R` and inner radius ~R*0.45.
const STAR_CX = NODE_W - 18
const STAR_CY = 16
const STAR_R = 9
const STAR_POINTS = (() => {
    const pts: string[] = []
    for (let i = 0; i < 10; i += 1) {
        const radius = i % 2 === 0 ? STAR_R : STAR_R * 0.45
        // Start at the top (angle = -90°), step 36° between alternating outer/inner vertices.
        const angle = ((i * 36 - 90) * Math.PI) / 180
        const x = STAR_CX + radius * Math.cos(angle)
        const y = STAR_CY + radius * Math.sin(angle)
        pts.push(`${x.toFixed(2)},${y.toFixed(2)}`)
    }
    return pts.join(' ')
})()
</script>

<template>
    <!--
        Per-user favourite toggle. Outline-only when unset; filled
        gold when marked. The hit target is a transparent square
        around the star so taps on tablets land reliably. Click
        bubbling is stopped in the handler so the card's onSelect
        doesn't ALSO fire on the same gesture.
    -->
    <g
        class="favourite"
        :class="{ filled: props.filled }"
        role="button"
        tabindex="0"
        :aria-label="favouriteTooltip"
        :data-testid="`tree-node-favourite-${props.nodeId}`"
        @click="onStarClick"
        @keydown.enter.stop="onStarClick"
        @keydown.space.stop.prevent="onStarClick"
    >
        <rect
            :x="STAR_CX - STAR_R - 3"
            :y="STAR_CY - STAR_R - 3"
            :width="(STAR_R + 3) * 2"
            :height="(STAR_R + 3) * 2"
            fill="transparent"
        />
        <polygon :points="STAR_POINTS" />
    </g>
</template>

<style scoped>
/* Favourite star — outline-only when unset (subtle dark stroke so it
 * reads as "available action" without dominating the card), filled
 * gold once marked. The hit-rect intercepts pointer events so the
 * polygon doesn't need to grow to be tappable. */
.favourite {
    cursor: pointer;
}
.favourite polygon {
    fill: none;
    /* Vuetify theme RGB vars are comma-separated triplets, so the
     * `rgb(... / a)` slash syntax is INVALID and silently drops the
     * declaration — which left the idle star with the SVG default
     * `stroke: none` (invisible until hover). Use the `rgba(var, a)`
     * comma form so the unmarked star is always faintly visible. */
    stroke: rgba(var(--v-theme-on-surface), 0.55);
    stroke-width: 1.75;
    stroke-linejoin: round;
    transition:
        fill 150ms ease-in-out,
        stroke 150ms ease-in-out,
        opacity 150ms ease-in-out;
    /* Always present, just muted; hover/marked states lift it. */
    opacity: 0.55;
}
.favourite:hover polygon,
.favourite:focus-visible polygon {
    stroke: var(--star);
    opacity: 1;
}
.favourite.filled polygon {
    fill: var(--star);
    stroke: var(--acc-strong);
    opacity: 1;
}
</style>
