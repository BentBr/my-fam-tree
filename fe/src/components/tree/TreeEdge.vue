<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
    kind: 'parent' | 'partner'
    ax: number
    ay: number
    bx: number
    by: number
    /**
     * Partnership kind from the BE — only meaningful when
     * `kind === 'partner'`. `'marriage'` shows two interlocked golden
     * rings at the midpoint; everything else (civil_union, partnership,
     * or `null` from fixtures) keeps the rose-pink heart. Ignored for
     * parent edges.
     */
    partnershipKind?: string | null
    /**
     * `true` when this partnership has ended (divorced, separated,
     * died). Greys the connecting line + midpoint glyph so the
     * relationship still reads on the canvas as "historical" instead
     * of disappearing. Ignored for parent edges.
     */
    ended?: boolean
    /**
     * `true` when the two partners sit adjacent on the canvas — no
     * intermediate same-row node hides the midpoint glyph. We skip
     * drawing the dashed connecting line in that case: the glyph
     * (heart or rings) sitting between the two nodes is the
     * connector, and the redundant line behind it is visual noise.
     *
     * `false` for "long" partnerships where the line is the ONLY
     * cue (the midpoint glyph would otherwise hide behind the
     * intermediate node). Computed in the layout pipeline; the
     * renderer just consumes it. Ignored for parent edges.
     */
    directlyAdjacent?: boolean
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
// horizontal lines so the relationship glyph sits naturally on the midpoint.
const path = computed(() => {
    if (props.kind === 'parent') {
        const midY = (props.ay + props.by) / 2
        return `M ${props.ax} ${props.ay} C ${props.ax} ${midY}, ${props.bx} ${midY}, ${props.bx} ${props.by}`
    }
    return `M ${props.ax} ${props.ay} L ${props.bx} ${props.by}`
})

const midX = computed(() => (props.ax + props.bx) / 2)
const midY = computed(() => (props.ay + props.by) / 2)

// A marriage gets the interlocked-rings glyph; civil unions, registered
// partnerships, or anything where `kind` is missing keep the legacy
// rose-pink heart. The `ended` flag is orthogonal — it greys whichever
// glyph is showing.
const isMarriage = computed(() => props.partnershipKind === 'marriage')
const isEnded = computed(() => props.ended === true)
// For directly-adjacent partner pairs (the glyph sits between the two
// nodes, fully visible), the dashed connecting line is redundant —
// suppress it and let the glyph be the connector. Parent edges always
// draw their line; this flag only affects partner edges.
const showPartnerLine = computed(() => props.kind !== 'partner' || props.directlyAdjacent !== true)

// Tailwind-style CSS class set so the stylesheet can pick the right
// stroke/fill via class scoping. Keeping the conditional in TS rather
// than in `:style` keeps the colour decision in one place (the CSS).
const partnerClasses = computed(() => ({
    marriage: isMarriage.value,
    ended: isEnded.value,
}))
</script>

<template>
    <g
        aria-hidden="true"
        :class="[
            'edge-group',
            kind,
            partnerClasses,
            { highlighted: props.isHighlighted === true, dimmed: props.isDimmed === true },
        ]"
        :data-testid="`tree-edge-${kind}`"
    >
        <path v-if="showPartnerLine" :d="path" class="edge" fill="none" />
        <!-- Midpoint glyph for partner edges. Marriage → two overlapping
             rings (golden, or grey when ended). Non-marriage → heart
             (rose-pink, or grey when ended). Pure SVG so it scales with
             the canvas zoom. -->
        <template v-if="kind === 'partner'">
            <g v-if="isMarriage" :transform="`translate(${midX - 6.5} ${midY - 5})`" class="rings">
                <!-- Two adjacent 3.5px-radius circles offset by 5px:
                     overlap reads as the universal wedding-rings symbol
                     at this scale. Stroke only, no fill. -->
                <circle cx="4" cy="5" r="3.25" fill="none" />
                <circle cx="9" cy="5" r="3.25" fill="none" />
            </g>
            <g v-else :transform="`translate(${midX - 6} ${midY - 6})`" class="heart">
                <path d="M6 11 L1 6 a3 3 0 0 1 5 -2 a3 3 0 0 1 5 2 z" />
            </g>
        </template>
    </g>
</template>

<style scoped>
.edge-group .edge {
    fill: none;
}
.edge-group.parent .edge {
    /* Warm-neutral connector. The token (`--edge`) flips between
     * light + dark in lock-step with the rest of the palette so the
     * tree reads correctly in both themes. */
    stroke: var(--edge);
    stroke-width: 2;
    stroke-linecap: round;
}
.edge-group.partner .edge {
    /* Rose-pink relationship colour — same token as the heart glyph
     * below so partner edges + the heart at the midpoint visually
     * agree. */
    stroke: var(--rel);
    stroke-width: 2;
    stroke-dasharray: 6 4;
}
/* Ended partnerships: still drawn so the relationship remains visible
 * on the canvas, but the colour drops to a muted grey so it reads as
 * "historical" — same treatment for the line AND the midpoint glyph
 * regardless of which glyph (heart or rings) is showing. */
.edge-group.partner.ended .edge {
    stroke: var(--rel-muted);
}
.heart path {
    fill: var(--rel);
    opacity: 0.85;
}
.edge-group.partner.ended .heart path {
    fill: var(--rel-muted);
    opacity: 0.9;
}
.rings circle {
    /* Stroke-only rings — no fill — for the marriage glyph. The gold
     * `--rings` token is themed (see tokens.css); ended marriages
     * inherit `.ended` further down and switch to muted grey. */
    stroke: var(--rings);
    stroke-width: 1.5;
    fill: none;
    opacity: 0.95;
}
.edge-group.partner.ended .rings circle {
    stroke: var(--rel-muted);
    opacity: 0.95;
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
    /* Keep the rose relationship colour but drop the dash so the line
     * connects the hovered partner pair without visual chopping. */
    stroke: var(--rel);
}
.edge-group.highlighted.partner.ended .edge {
    /* Hover on an ended partnership: keep the muted grey but still
     * thicken + de-dash so the user can trace the hovered pair. */
    stroke: var(--rel-muted);
}
.edge-group.highlighted .heart path,
.edge-group.highlighted .rings circle {
    opacity: 1;
}

/* Unrelated edges fade so the highlighted ones pop. Same opacity floor
 * as TreeNode.dimmed so the visual treatment matches. */
.edge-group.dimmed {
    opacity: 0.4;
    transition: opacity 150ms ease-in-out;
}
</style>
