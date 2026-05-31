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

/**
 * Lineage-relation id set for the hovered node. Walks the parent_ids
 * graph upward to gather every ancestor and downward to gather every
 * descendant, then folds in partners (just the direct ones — a
 * spouse's lineage is their own story). The hovered node itself is
 * excluded from the set so it can still get the `.hovered` treatment
 * separately from `.related`.
 *
 * Inputs the layout already gives us:
 *   - `n.parent_ids`  → walk up
 *   - persons whose `parent_ids.includes(id)` → walk down
 *   - `n.partner_ids` → adjacent
 *
 * Cycle-safe: the visited set short-circuits any back-edges so a
 * malformed graph can never spin forever.
 */
const relatedIds = computed<Set<string>>(() => {
    const id = hoverId.value
    if (id === null) return new Set<string>()
    const target = props.tree.nodes.find((n) => n.id === id)
    if (target === undefined) return new Set<string>()

    // Build a quick parent → child index once so the descendant walk is
    // linear per node, not O(N) per step.
    const childrenOf = new Map<string, string[]>()
    for (const n of props.tree.nodes) {
        for (const p of n.parent_ids) {
            const bucket = childrenOf.get(p)
            if (bucket === undefined) {
                childrenOf.set(p, [n.id])
            } else {
                bucket.push(n.id)
            }
        }
    }
    const nodeById = new Map(props.tree.nodes.map((n) => [n.id, n]))

    const visited = new Set<string>([id])
    const out = new Set<string>()

    // Ancestors — repeated `parent_ids` until exhausted.
    const upQueue: string[] = [...target.parent_ids]
    while (upQueue.length > 0) {
        const cur = upQueue.shift() as string
        if (visited.has(cur)) continue
        visited.add(cur)
        out.add(cur)
        const node = nodeById.get(cur)
        if (node !== undefined) upQueue.push(...node.parent_ids)
    }

    // Descendants — BFS over the parent→child index.
    const downQueue: string[] = [...(childrenOf.get(id) ?? [])]
    while (downQueue.length > 0) {
        const cur = downQueue.shift() as string
        if (visited.has(cur)) continue
        visited.add(cur)
        out.add(cur)
        downQueue.push(...(childrenOf.get(cur) ?? []))
    }

    // Direct partners (not their lineage — that's a different family).
    for (const pid of target.partner_ids) out.add(pid)

    out.delete(id)
    return out
})

function onNodeHover(id: string | null): void {
    hoverId.value = id
}

/**
 * Whether an edge connects two nodes inside the hovered lineage —
 * i.e. both endpoints are either the hovered node itself or in the
 * related-id set (ancestors, descendants, or partners). That way the
 * full chain of inheritance highlights together, not just the single
 * hop next to the hovered card.
 */
function isEdgeHighlighted(aId: string, bId: string): boolean {
    const id = hoverId.value
    if (id === null) return false
    const aIn = aId === id || relatedIds.value.has(aId)
    const bIn = bId === id || relatedIds.value.has(bId)
    return aIn && bIn
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
defineExpose({ refit: () => fitToView(true) })
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
