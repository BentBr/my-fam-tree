<script setup lang="ts">
import { easeCubicInOut } from 'd3-ease'
import { select } from 'd3-selection'
// d3-transition has no exports we use directly — importing it for side-effects
// augments `d3-selection`'s `Selection.prototype` with `.transition()`, which
// d3-zoom's animated `.call(zoom.transform, …)` path relies on.
import 'd3-transition'
import { zoom as d3zoom, zoomIdentity, type ZoomBehavior, type ZoomTransform } from 'd3-zoom'
import { computed, onMounted, ref, watch } from 'vue'

import { layoutTree, NODE_H, NODE_W, type Positioned, type TreeInput } from './layout'
import TreeEdge from './TreeEdge.vue'
import TreeNode from './TreeNode.vue'

const props = defineProps<{
    tree: TreeInput
    selectedId: string | null
    /** When set, centers the viewport on this person on mount and on subsequent change. */
    centerOnId: string | null
    /**
     * `users.id` of the signed-in user. Compared against each TreeNode's
     * `linked_user_id` to flag the "this is you" card. Decoupled from
     * `centerOnId`: an explicit `?center=` deep-link or persisted focus
     * may point to a different person, but the user-highlight should
     * always track the signed-in user.
     */
    currentUserId: string | null
}>()

const emit = defineEmits<{
    (e: 'select', id: string): void
}>()

const svgEl = ref<SVGSVGElement | null>(null)
const gEl = ref<SVGGElement | null>(null)
const wrapEl = ref<HTMLDivElement | null>(null)
let zoomBehavior: ZoomBehavior<SVGSVGElement, unknown> | null = null

const layout = computed(() => layoutTree(props.tree))

function nodeCenter(id: string): { x: number; y: number } | null {
    const n = layout.value.nodes.find((p: Positioned) => p.id === id)
    if (n === undefined) return null
    return { x: n.x + NODE_W / 2, y: n.y + NODE_H / 2 }
}

/** Compute the scale that fits the whole layout into the viewport. */
function fitScale(): number {
    const wrap = wrapEl.value
    if (wrap === null) return 1
    const w = wrap.clientWidth
    const h = wrap.clientHeight
    const contentW = Math.max(layout.value.width, 1)
    const contentH = Math.max(layout.value.height, 1)
    const padding = 60
    const scaleX = (w - padding * 2) / contentW
    const scaleY = (h - padding * 2) / contentH
    const raw = Math.min(scaleX, scaleY, 1)
    return Math.max(raw, 0.25)
}

function centerOn(id: string, animate: boolean): void {
    const c = nodeCenter(id)
    const svg = svgEl.value
    const wrap = wrapEl.value
    if (c === null || svg === null || zoomBehavior === null || wrap === null) return
    const w = wrap.clientWidth
    const h = wrap.clientHeight
    // Re-use the fit scale rather than snapping to 1×. At 1× a non-trivial
    // tree no longer fits the viewport and the rest of the family slides off
    // the canvas — which was the prior on-load regression. Same scale ⇒ a
    // pan, not a zoom-and-clip.
    const scale = fitScale()
    const transform = zoomIdentity.translate(w / 2 - c.x * scale, h / 2 - c.y * scale).scale(scale)
    const sel = select(svg)
    if (animate) {
        // Animated centering reads as the canvas easing toward the new focus —
        // visually similar to "the tree leans toward this person".
        sel.transition().duration(600).ease(easeCubicInOut).call(zoomBehavior.transform, transform)
    } else {
        sel.call(zoomBehavior.transform, transform)
    }
}

/**
 * Compute a transform that fits the full layout bounding box into the
 * viewport with a small padding. Used on initial mount so the user
 * sees every person at once instead of an arbitrary corner of the tree.
 * `scaleExtent` clamps the result so very small / very large trees still
 * render at a readable scale rather than a postage stamp / a blur.
 */
function fitToView(animate: boolean): void {
    const svg = svgEl.value
    const wrap = wrapEl.value
    if (svg === null || zoomBehavior === null || wrap === null) return
    const w = wrap.clientWidth
    const h = wrap.clientHeight
    const contentW = Math.max(layout.value.width, 1)
    const contentH = Math.max(layout.value.height, 1)
    const padding = 60
    const scaleX = (w - padding * 2) / contentW
    const scaleY = (h - padding * 2) / contentH
    const scale = Math.min(scaleX, scaleY, 1)
    const clamped = Math.max(scale, 0.25)
    const tx = (w - contentW * clamped) / 2
    const ty = (h - contentH * clamped) / 2
    const transform = zoomIdentity.translate(tx, ty).scale(clamped)
    const sel = select(svg)
    if (animate) {
        sel.transition().duration(600).ease(easeCubicInOut).call(zoomBehavior.transform, transform)
    } else {
        sel.call(zoomBehavior.transform, transform)
    }
}

onMounted(() => {
    const svg = svgEl.value
    const g = gEl.value
    if (svg === null || g === null) return
    zoomBehavior = d3zoom<SVGSVGElement, unknown>()
        .scaleExtent([0.25, 3])
        .on('zoom', (event: { transform: ZoomTransform }) => {
            g.setAttribute('transform', event.transform.toString())
        })
    select(svg).call(zoomBehavior)

    // Initial layout: fit-to-view first so the whole tree paints (the
    // earlier regression was: snap to 1× and shove ancestors off-canvas).
    // Then, if we have a center target (signed-in user by default), pan to
    // it — `centerOn` now reuses the fit scale, so this is a pan, not a
    // zoom. Result: the tree fits AND the user's node lands at viewport
    // center.
    fitToView(false)
    if (props.centerOnId !== null) {
        centerOn(props.centerOnId, false)
    }
})

watch(
    () => props.centerOnId,
    (id) => {
        if (id !== null) centerOn(id, true)
    },
)

// Refit when the node count changes (person added / removed). Without
// this, a freshly-added person can land outside the current viewport
// and look like the tree didn't update — even though the SVG did.
watch(
    () => layout.value.nodes.length,
    () => {
        if (props.centerOnId === null) fitToView(true)
    },
)

// Imperative refit handle for the parent view's "Fit to view" toolbar
// button. Lets the user recover from any panned/zoomed state with one
// click instead of having to scroll the canvas back themselves.
defineExpose({ refit: () => fitToView(true) })
</script>

<template>
    <div ref="wrapEl" class="tree-wrap" data-testid="tree-canvas">
        <svg
            ref="svgEl"
            :viewBox="`0 0 ${Math.max(layout.width + 80, 1)} ${Math.max(layout.height + 80, 1)}`"
            preserveAspectRatio="xMidYMid meet"
            role="application"
            aria-label="Family tree"
        >
            <defs>
                <filter id="treeNodeShadow" x="-30%" y="-30%" width="160%" height="160%">
                    <feDropShadow dx="0" dy="2" stdDeviation="3" flood-opacity="0.08" />
                </filter>
                <filter id="treeNodeHoverShadow" x="-50%" y="-50%" width="200%" height="200%">
                    <feDropShadow
                        dx="0"
                        dy="8"
                        stdDeviation="6"
                        flood-color="rgb(var(--v-theme-primary))"
                        flood-opacity="0.22"
                    />
                </filter>
            </defs>
            <g ref="gEl">
                <TreeEdge
                    v-for="e in layout.parentEdges"
                    :key="`p-${e.childId}-${e.parentId}`"
                    kind="parent"
                    :ax="e.childX"
                    :ay="e.childY"
                    :bx="e.parentX"
                    :by="e.parentY"
                />
                <TreeEdge
                    v-for="e in layout.partnerEdges"
                    :key="`pn-${e.aId}-${e.bId}`"
                    kind="partner"
                    :ax="e.ax"
                    :ay="e.ay"
                    :bx="e.bx"
                    :by="e.by"
                />
                <TreeNode
                    v-for="n in layout.nodes"
                    :key="n.id"
                    :node="n"
                    :selected="n.id === selectedId"
                    :is-current-user="n.linked_user_id !== null && n.linked_user_id === props.currentUserId"
                    @select="(id: string) => emit('select', id)"
                />
            </g>
        </svg>
    </div>
</template>

<style scoped>
/* Fill the viewport; touch-action: none stops the browser from intercepting
 * pinch/pan so d3-zoom owns the gesture (mandatory for tablets). */
.tree-wrap {
    width: 100%;
    height: calc(100vh - 140px);
    border-radius: 12px;
    background:
        radial-gradient(800px 400px at 50% -10%, rgb(var(--v-theme-primary) / 0.06), transparent 60%),
        rgb(var(--v-theme-surface));
    border: 1px solid rgb(var(--v-theme-on-surface) / 0.08);
    overflow: hidden;
    touch-action: none;
    user-select: none;
}
svg {
    width: 100%;
    height: 100%;
    cursor: grab;
    display: block;
}
svg:active {
    cursor: grabbing;
}
</style>
