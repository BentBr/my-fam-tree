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

function centerOn(id: string, animate: boolean): void {
    const c = nodeCenter(id)
    const svg = svgEl.value
    const wrap = wrapEl.value
    if (c === null || svg === null || zoomBehavior === null || wrap === null) return
    const w = wrap.clientWidth
    const h = wrap.clientHeight
    const scale = 1
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

    // Initial centering: explicit centerOnId wins; otherwise nudge the layout
    // so the top-left of the canvas sits at a comfortable gutter.
    if (props.centerOnId !== null) {
        centerOn(props.centerOnId, false)
    } else {
        const sel = select(svg)
        sel.call(zoomBehavior.transform, zoomIdentity.translate(40, 40))
    }
})

watch(
    () => props.centerOnId,
    (id) => {
        if (id !== null) centerOn(id, true)
    },
)
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
