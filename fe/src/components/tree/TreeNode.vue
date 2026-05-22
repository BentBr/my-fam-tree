<script setup lang="ts">
import { computed, ref } from 'vue'

import { NODE_H, NODE_W, type Positioned } from './layout'

const props = defineProps<{
    node: Positioned
    selected: boolean
    isCurrentUser?: boolean
}>()

const isDeceased = (): boolean => props.node.death_date !== null && props.node.death_date !== ''

const emit = defineEmits<{
    (e: 'select', id: string): void
}>()

const hovered = ref(false)

// Width budget for the text columns (right of the avatar circle). Used to
// truncate the full name with ellipsis when it would overflow the card.
const TEXT_LEFT = 64
const TEXT_RIGHT_PAD = 12
const TEXT_WIDTH = NODE_W - TEXT_LEFT - TEXT_RIGHT_PAD

// Average glyph width for `system-ui` at the sizes we use. Cheap heuristic
// — better than measuring per-glyph and good enough for ellipsizing names
// that overflow the card. The card is 220px wide; the budget here lands
// around 18-22 chars for the name and ~24 for the smaller dates row.
const NAME_AVG_CHAR_PX = 7.5
const DATES_AVG_CHAR_PX = 5.8

function truncate(s: string, maxChars: number): string {
    if (s.length <= maxChars) return s
    if (maxChars <= 1) return '…'
    return `${s.slice(0, maxChars - 1)}…`
}

const fullName = computed(() => {
    const raw = `${props.node.given_name} ${props.node.family_name}`.trim()
    return truncate(raw, Math.floor(TEXT_WIDTH / NAME_AVG_CHAR_PX))
})

const DATES_MAX_CHARS = Math.floor(TEXT_WIDTH / DATES_AVG_CHAR_PX)

const birthLabel = computed(() => {
    const b = props.node.birth_date ?? ''
    if (b === '') return ''
    return truncate(`* ${b}`, DATES_MAX_CHARS)
})

const deathLabel = computed(() => {
    const d = props.node.death_date ?? ''
    if (d === '') return ''
    return truncate(`† ${d}`, DATES_MAX_CHARS)
})

const hasDates = computed(() => birthLabel.value !== '' || deathLabel.value !== '')

function initials(p: Positioned): string {
    const a = p.given_name.charAt(0)
    const b = p.family_name.charAt(0)
    const combined = `${a}${b}`.toUpperCase()
    return combined === '' ? '?' : combined
}

function onSelect(): void {
    emit('select', props.node.id)
}
</script>

<template>
    <g
        role="button"
        tabindex="0"
        :aria-label="`${props.node.given_name} ${props.node.family_name}, born ${props.node.birth_date ?? 'unknown'}`"
        :class="[
            'tree-node',
            {
                selected: props.selected,
                hovered,
                'current-user': props.isCurrentUser === true,
                deceased: isDeceased(),
            },
        ]"
        :transform="`translate(${props.node.x}, ${props.node.y})`"
        :data-testid="`tree-node-${props.node.id}`"
        :filter="hovered || props.selected ? 'url(#treeNodeHoverShadow)' : 'url(#treeNodeShadow)'"
        @click="onSelect"
        @keydown.enter="onSelect"
        @keydown.space.prevent="onSelect"
        @mouseenter="hovered = true"
        @mouseleave="hovered = false"
    >
        <rect :width="NODE_W" :height="NODE_H" rx="12" />
        <circle :cx="32" :cy="NODE_H / 2" r="22" class="avatar" />
        <text :x="32" :y="NODE_H / 2 + 6" text-anchor="middle" class="initials">
            {{ initials(props.node) }}
        </text>
        <!--
            Native SVG <text> rather than <foreignObject>+<div>. Chromium
            inconsistently applies parent <g> transforms to HTML content
            inside <foreignObject>, which made nodes vanish under the
            fit-to-view zoom. SVG <text> scales uniformly with the canvas.
        -->
        <text :x="TEXT_LEFT" :y="hasDates ? NODE_H / 2 - 6 : NODE_H / 2 + 5" class="name" data-testid="tree-node-name">
            {{ fullName }}
        </text>
        <text
            v-if="birthLabel !== ''"
            :x="TEXT_LEFT"
            :y="NODE_H / 2 + 10"
            class="dates dates-birth"
            data-testid="tree-node-birth"
        >
            {{ birthLabel }}
        </text>
        <text
            v-if="deathLabel !== ''"
            :x="TEXT_LEFT"
            :y="NODE_H / 2 + 24"
            class="dates dates-death"
            data-testid="tree-node-death"
        >
            {{ deathLabel }}
        </text>
    </g>
</template>

<style scoped>
.tree-node {
    cursor: pointer;
    /* Soft fade-in whenever the node mounts (re-layout after add/remove).
     * NB: we animate ONLY opacity here. CSS `transform` on an SVG <g>
     * overrides the `transform` *attribute* set by Vue's :transform binding,
     * collapsing every node to (0,0) of the parent group. We learned that
     * the hard way — a scale(0.96 → 1) keyframe made the entire tree look
     * like a single floating card on first paint. */
    animation: tree-node-in 300ms ease-out both;
}
@keyframes tree-node-in {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

.tree-node rect {
    fill: rgb(var(--v-theme-surface));
    stroke: rgb(var(--v-theme-on-surface) / 0.18);
    stroke-width: 1;
    transition:
        stroke 150ms ease-in-out,
        stroke-width 150ms ease-in-out;
}
.tree-node.selected rect,
.tree-node:focus-visible rect {
    stroke: rgb(var(--v-theme-primary));
    stroke-width: 2;
}
.tree-node.hovered rect {
    stroke: rgb(var(--v-theme-primary) / 0.6);
}

.avatar {
    fill: rgb(var(--v-theme-primary) / 0.12);
    stroke: rgb(var(--v-theme-primary) / 0.35);
    stroke-width: 1;
}
.initials {
    font:
        600 14px system-ui,
        sans-serif;
    fill: rgb(var(--v-theme-primary));
    pointer-events: none;
}
.name {
    font:
        600 13px system-ui,
        sans-serif;
    fill: rgb(var(--v-theme-on-surface));
    pointer-events: none;
}
.dates {
    font:
        11px system-ui,
        sans-serif;
    fill: rgb(var(--v-theme-on-surface) / 0.6);
    pointer-events: none;
}
/* Death date sits on its own row below birth and reads smaller + fainter
 * so the "*" / "†" lines pair visually like a small obituary detail. */
.dates-death {
    font:
        9.5px system-ui,
        sans-serif;
    fill: rgb(var(--v-theme-on-surface) / 0.45);
}

/* Baseline "this is you" treatment: warm amber ring + soft amber wash,
 * distinct from the primary-blue used for selected/hovered cards so the
 * user marker doesn't read as a transient state. The final design will
 * iterate; this only needs to be unmistakable at a glance. */
.tree-node.current-user rect {
    stroke: #f59e0b;
    stroke-width: 2.5;
    fill: #fffbeb;
}
.tree-node.current-user.selected rect,
.tree-node.current-user:focus-visible rect {
    stroke-width: 3;
}

/* Deceased cards get a soft grey wash so they read as historical at a
 * glance. The avatar+text inside the SVG can't use `filter: grayscale` —
 * SVG filters require a <filter> def — so we tint the rect fill and the
 * stroke instead, and lighten the text/initials. */
.tree-node.deceased rect {
    fill: #f1f5f9;
    stroke: #cbd5e1;
}
.tree-node.deceased .avatar {
    fill: #e2e8f0;
    stroke: #94a3b8;
}
.tree-node.deceased .initials {
    fill: #64748b;
}
.tree-node.deceased .name,
.tree-node.deceased .dates {
    fill: #64748b;
}
</style>
