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
import { computeLineageIds, isLineageEdge } from './useLineage'

const props = withDefaults(
    defineProps<{
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
        /**
         * "Highlight mode": when true, clicking a person PINS them (and their
         * direct lineage stays lit) instead of opening the detail drawer, so
         * the user can pan/zoom around and keep tracing the line. Hover is
         * ignored while pinned so the highlight survives scrolling. When the
         * mode flips off the pin is cleared.
         */
        highlightMode?: boolean
    }>(),
    { highlightMode: false },
)

const emit = defineEmits<{
    (e: 'select', id: string): void
    (e: 'toggle-favourite', id: string, next: boolean): void
}>()

const svgEl = ref<SVGSVGElement | null>(null)
const gEl = ref<SVGGElement | null>(null)
const wrapEl = ref<HTMLDivElement | null>(null)
let zoomBehavior: ZoomBehavior<SVGSVGElement, unknown> | null = null

const layout = computed(() => layoutTree(props.tree))

// Currently-hovered tree node id (null when the pointer isn't over any node).
// Driven by the `hover` event each `TreeNode` emits on mouseenter / mouseleave.
const hoverId = ref<string | null>(null)

// Highlight-mode pin: the person clicked while highlight mode is on. Their
// lineage stays lit until they're clicked again (toggle off) or a different
// person is clicked. Cleared whenever the mode flips off.
const pinnedId = ref<string | null>(null)
watch(
    () => props.highlightMode,
    (on) => {
        if (!on) pinnedId.value = null
    },
)

/**
 * The person whose lineage is currently lit. In highlight mode that's
 * the PINNED person (sticky — hover is ignored so the highlight survives
 * panning). Otherwise it tracks the hovered person.
 */
const focusId = computed<string | null>(() => (props.highlightMode ? pinnedId.value : hoverId.value))

/**
 * Lineage-relation id set for the focus person (ancestors + descendants
 * + direct partners; focus id excluded). See `computeLineageIds`.
 */
const relatedIds = computed<Set<string>>(() => computeLineageIds(props.tree.nodes, focusId.value))

function onNodeHover(id: string | null): void {
    hoverId.value = id
}

/**
 * Node-select dispatch. In highlight mode a click PINS the lineage
 * (toggling off if the same node is re-clicked) and does NOT open the
 * detail drawer. Otherwise it forwards `select` as before.
 */
function onNodeSelect(id: string): void {
    if (props.highlightMode) {
        pinnedId.value = pinnedId.value === id ? null : id
        return
    }
    emit('select', id)
}

/** Whether an edge lies inside the focus person's lineage. */
function isEdgeHighlighted(aId: string, bId: string): boolean {
    return isLineageEdge(focusId.value, relatedIds.value, aId, bId)
}

function nodeCenter(id: string): { x: number; y: number } | null {
    const n = layout.value.nodes.find((p: Positioned) => p.id === id)
    if (n === undefined) return null
    return { x: n.x + NODE_W / 2, y: n.y + NODE_H / 2 }
}

/**
 * Lower bound on the "fit to view" scale. A non-trivial seeded tree
 * (~20 persons spread across 4 generations) makes the pure fit-scale
 * collapse to ~0.3, which renders the cards as unreadable thumbnails.
 * Clamping at 0.5 keeps the cards legible at the cost of the viewport
 * sometimes clipping the outer cousins — the user can pan to them.
 *
 * Used ONLY by `fitToView` and the auto-fit fallback at mount; the user-
 * driven zoom range (`MIN_USER_SCALE`) goes further so manual zoom-out
 * isn't artificially capped at the fit-floor.
 */
const MIN_FIT_SCALE = 0.5

/**
 * Lower bound on the user-driven zoom (the d3-zoom `scaleExtent`). Below
 * the fit-clamp so users can intentionally pull back far enough to see a
 * large family at a glance even when the auto-fit refused to. The
 * "Fit to view" button keeps `MIN_FIT_SCALE` as its floor — pressing
 * fit on a huge tree still snaps to a legible scale rather than the
 * postage-stamp extreme. 0.1 lets a ~200-person tree fit on one
 * screen at typical desktop widths.
 */
const MIN_USER_SCALE = 0.1

/**
 * Initial focus scale when a `currentUserId` resolves to a node on mount.
 * 0.75 is "close enough that the user's card and the surrounding 1-2
 * generations are easily readable" without losing the wider-family context.
 */
const FOCUS_SCALE = 0.75

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
    return Math.max(raw, MIN_FIT_SCALE)
}

/**
 * Apply an absolute zoom transform either instantly or via a 600ms ease.
 * Centralised so all three callers (centerOn, fitToView, refit) share the
 * same animation curve and don't duplicate the `d3-transition` setup.
 */
function applyTransform(
    sel: ReturnType<typeof select<SVGSVGElement, unknown>>,
    transform: ReturnType<typeof zoomIdentity.translate>,
    animate: boolean,
): void {
    const zoom = zoomBehavior
    if (zoom === null) return
    if (animate) {
        sel.transition()
            .duration(600)
            .ease(easeCubicInOut)
            .call((t) => {
                zoom.transform(t, transform)
            })
    } else {
        zoom.transform(sel, transform)
    }
}

function centerOn(id: string, animate: boolean, scale?: number): void {
    const c = nodeCenter(id)
    const svg = svgEl.value
    const wrap = wrapEl.value
    if (c === null || svg === null || zoomBehavior === null || wrap === null) return
    const w = wrap.clientWidth
    const h = wrap.clientHeight
    // Default to the fit-scale clamp so a pure pan from the toolbar's
    // "center on me" path doesn't suddenly change zoom level. Callers that
    // want the initial-focus zoom (mount) pass `FOCUS_SCALE` explicitly.
    const s = scale ?? fitScale()
    const transform = zoomIdentity.translate(w / 2 - c.x * s, h / 2 - c.y * s).scale(s)
    applyTransform(select(svg), transform, animate)
}

/**
 * Compute a transform that fits the full layout bounding box into the
 * viewport with a small padding. Used by the "Fit to view" toolbar button
 * (and as the mount fallback when there is no `currentUserId` to focus on).
 * The scale is clamped from below at `MIN_FIT_SCALE` so the result stays
 * legible even on huge trees — better to clip a few outliers than render
 * every card as a postage stamp.
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
    const clamped = Math.max(scale, MIN_FIT_SCALE)
    const tx = (w - contentW * clamped) / 2
    const ty = (h - contentH * clamped) / 2
    const transform = zoomIdentity.translate(tx, ty).scale(clamped)
    applyTransform(select(svg), transform, animate)
}

onMounted(() => {
    const svg = svgEl.value
    const g = gEl.value
    if (svg === null || g === null) return
    zoomBehavior = d3zoom<SVGSVGElement, unknown>()
        .scaleExtent([MIN_USER_SCALE, 3])
        .on('zoom', (event: { transform: ZoomTransform }) => {
            g.setAttribute('transform', event.transform.toString())
        })
    select(svg).call(zoomBehavior)

    // Initial focus. v3 default: if the signed-in user resolves to a node
    // on the canvas, pan there at `FOCUS_SCALE` so the cards are legible
    // (the prior fitToView-on-mount path collapsed to ~0.3 on the 20-person
    // seed and the cards were unreadable thumbnails). When no user-linked
    // node exists, fall back to a clamped fit-to-view so the whole tree
    // still paints but at a legible minimum scale.
    const userNode = props.currentUserId === null ? null : nodeCenter(props.currentUserId)
    if (userNode !== null && props.currentUserId !== null) {
        centerOn(props.currentUserId, false, FOCUS_SCALE)
    } else if (props.centerOnId !== null) {
        centerOn(props.centerOnId, false, FOCUS_SCALE)
    } else {
        fitToView(false)
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
defineExpose({
    refit: () => {
        fitToView(true)
    },
})
</script>

<template>
    <div ref="wrapEl" class="tree-wrap" data-testid="tree-canvas">
        <svg ref="svgEl" role="application" aria-label="Family tree">
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
                    :is-highlighted="isEdgeHighlighted(e.childId, e.parentId)"
                    :is-dimmed="focusId !== null && !isEdgeHighlighted(e.childId, e.parentId)"
                />
                <TreeEdge
                    v-for="e in layout.partnerEdges"
                    :key="`pn-${e.aId}-${e.bId}`"
                    kind="partner"
                    :ax="e.ax"
                    :ay="e.ay"
                    :bx="e.bx"
                    :by="e.by"
                    :partnership-kind="e.kind"
                    :ended="e.ended"
                    :directly-adjacent="e.directlyAdjacent"
                    :is-highlighted="isEdgeHighlighted(e.aId, e.bId)"
                    :is-dimmed="focusId !== null && !isEdgeHighlighted(e.aId, e.bId)"
                />
                <TreeNode
                    v-for="n in layout.nodes"
                    :key="n.id"
                    :node="n"
                    :selected="n.id === selectedId"
                    :is-current-user="n.linked_user_id !== null && n.linked_user_id === props.currentUserId"
                    :is-hovered="n.id === focusId"
                    :is-related="relatedIds.has(n.id)"
                    :is-dimmed="focusId !== null && n.id !== focusId && !relatedIds.has(n.id)"
                    @select="(id: string) => onNodeSelect(id)"
                    @hover="(id: string | null) => onNodeHover(id)"
                    @toggle-favourite="(id: string, next: boolean) => emit('toggle-favourite', id, next)"
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
