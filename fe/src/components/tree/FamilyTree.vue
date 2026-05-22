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

// Currently-hovered tree node id (null when the pointer isn't over any node).
// Driven by the `hover` event each `TreeNode` emits on mouseenter / mouseleave.
const hoverId = ref<string | null>(null)

/**
 * Direct-relation id set for the hovered node — union of:
 *   - the hovered person's `parent_ids` (their parents),
 *   - the persons whose `parent_ids` include the hovered person (their children),
 *   - the hovered person's `partner_ids` (their partners).
 *
 * Empty when nothing is hovered. The set never includes the hovered node
 * itself; that one gets the `.hovered` treatment instead of `.related`.
 */
const relatedIds = computed<Set<string>>(() => {
    const id = hoverId.value
    if (id === null) return new Set<string>()
    const target = props.tree.nodes.find((n) => n.id === id)
    if (target === undefined) return new Set<string>()
    const out = new Set<string>()
    for (const pid of target.parent_ids) out.add(pid)
    for (const pid of target.partner_ids) out.add(pid)
    for (const n of props.tree.nodes) {
        if (n.parent_ids.includes(id)) out.add(n.id)
    }
    out.delete(id)
    return out
})

function onNodeHover(id: string | null): void {
    hoverId.value = id
}

/**
 * Whether an edge connects the hovered node to one of its direct relations.
 * Parent edges: hovered node is either the child or the parent and the
 * counterpart is the hovered node's parent / child respectively. Partner
 * edges: hovered node is one of the two members.
 */
function isEdgeHighlighted(aId: string, bId: string): boolean {
    const id = hoverId.value
    if (id === null) return false
    return aId === id || bId === id
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
 */
const MIN_FIT_SCALE = 0.5

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
    if (zoomBehavior === null) return
    if (animate) {
        sel.transition().duration(600).ease(easeCubicInOut).call(zoomBehavior.transform, transform)
    } else {
        sel.call(zoomBehavior.transform, transform)
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
        .scaleExtent([MIN_FIT_SCALE, 3])
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
                    :is-highlighted="isEdgeHighlighted(e.childId, e.parentId)"
                    :is-dimmed="hoverId !== null && !isEdgeHighlighted(e.childId, e.parentId)"
                />
                <TreeEdge
                    v-for="e in layout.partnerEdges"
                    :key="`pn-${e.aId}-${e.bId}`"
                    kind="partner"
                    :ax="e.ax"
                    :ay="e.ay"
                    :bx="e.bx"
                    :by="e.by"
                    :is-highlighted="isEdgeHighlighted(e.aId, e.bId)"
                    :is-dimmed="hoverId !== null && !isEdgeHighlighted(e.aId, e.bId)"
                />
                <TreeNode
                    v-for="n in layout.nodes"
                    :key="n.id"
                    :node="n"
                    :selected="n.id === selectedId"
                    :is-current-user="n.linked_user_id !== null && n.linked_user_id === props.currentUserId"
                    :is-hovered="n.id === hoverId"
                    :is-related="relatedIds.has(n.id)"
                    :is-dimmed="hoverId !== null && n.id !== hoverId && !relatedIds.has(n.id)"
                    @select="(id: string) => emit('select', id)"
                    @hover="(id: string | null) => onNodeHover(id)"
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
