<script setup lang="ts">
import { computed } from 'vue'

import { NODE_H, NODE_W, type Positioned } from './layout'

const props = defineProps<{
    node: Positioned
    selected: boolean
    isCurrentUser?: boolean
    /** Whether the parent canvas currently has this node as the hover target. */
    isHovered?: boolean
    /** Whether the parent canvas has a hover target and THIS node is one of
     * its direct relations (parent / child / partner). Gets the softer
     * `related` treatment — slightly thicker blue-tinted stroke. */
    isRelated?: boolean
    /** Whether the parent canvas has a hover target and THIS node is neither
     * the target nor a direct relation. Fades to opacity 0.4 so the
     * highlighted subset reads cleanly. */
    isDimmed?: boolean
}>()

const isDeceased = (): boolean => props.node.death_date !== null && props.node.death_date !== ''

const emit = defineEmits<{
    (e: 'select', id: string): void
    (e: 'hover', id: string | null): void
}>()

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

/**
 * Parse a (possibly partial) ISO date string into a Date. Accepts the
 * full `YYYY-MM-DD` shape, `YYYY-MM`, and bare `YYYY`. Returns null on
 * anything else. We hand-parse rather than feeding partial ISO to the
 * `Date` constructor — Safari rejects bare `YYYY-MM` and Chromium
 * interprets it as UTC midnight which can land on the previous local
 * day depending on the runtime tz.
 */
function parseIsoDate(s: string): Date | null {
    const parts = s.match(/^(\d{4})(?:-(\d{2}))?(?:-(\d{2}))?/)
    if (parts === null) return null
    const head = parts[1]
    if (head === undefined) return null
    const yr = Number.parseInt(head, 10)
    if (!Number.isFinite(yr)) return null
    const mo = parts[2] !== undefined ? Number.parseInt(parts[2], 10) - 1 : 0
    const da = parts[3] !== undefined ? Number.parseInt(parts[3], 10) : 1
    return new Date(yr, mo, da)
}

/**
 * Full years between two dates, day-precision. Canonical "how old are
 * you" semantics — subtracts a year if the end month/day hasn't reached
 * the start month/day yet.
 */
function fullYearsBetween(from: Date, to: Date): number {
    let years = to.getFullYear() - from.getFullYear()
    if (to.getMonth() < from.getMonth() || (to.getMonth() === from.getMonth() && to.getDate() < from.getDate())) {
        years -= 1
    }
    return years
}

/**
 * Age label shown right-aligned on the birth date row. Living: just the
 * number. Deceased: `N (†)` to mark age-at-death. Returns the empty
 * string when birth_date is missing or unparseable — no cell rendered.
 */
const ageLabel = computed(() => {
    const birth = props.node.birth_date ?? ''
    if (birth === '') return ''
    const birthDate = parseIsoDate(birth)
    if (birthDate === null) return ''
    const deathStr = props.node.death_date ?? ''
    if (deathStr === '') {
        const years = fullYearsBetween(birthDate, new Date())
        return years >= 0 ? String(years) : ''
    }
    const deathDate = parseIsoDate(deathStr)
    if (deathDate === null) return ''
    const years = fullYearsBetween(birthDate, deathDate)
    return years >= 0 ? `${years} (†)` : ''
})

function initials(p: Positioned): string {
    const a = p.given_name.charAt(0)
    const b = p.family_name.charAt(0)
    const combined = `${a}${b}`.toUpperCase()
    return combined === '' ? '?' : combined
}

function onSelect(): void {
    emit('select', props.node.id)
}

function onHoverEnter(): void {
    emit('hover', props.node.id)
}

function onHoverLeave(): void {
    emit('hover', null)
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
                hovered: props.isHovered === true,
                related: props.isRelated === true,
                dimmed: props.isDimmed === true,
                'current-user': props.isCurrentUser === true,
                deceased: isDeceased(),
            },
        ]"
        :transform="`translate(${props.node.x}, ${props.node.y})`"
        :data-testid="`tree-node-${props.node.id}`"
        :filter="props.isHovered === true || props.selected ? 'url(#treeNodeHoverShadow)' : 'url(#treeNodeShadow)'"
        @click="onSelect"
        @keydown.enter="onSelect"
        @keydown.space.prevent="onSelect"
        @mouseenter="onHoverEnter"
        @mouseleave="onHoverLeave"
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
        <!--
            Age cell, right-aligned on the birth row. Living shows current
            age in full years; deceased shows age at death suffixed with
            "(†)" so it's clear we're not still counting. Hidden when the
            birth date is missing/unparseable — no row, no zero. -->
        <text
            v-if="ageLabel !== ''"
            :x="NODE_W - TEXT_RIGHT_PAD"
            :y="NODE_H / 2 + 10"
            text-anchor="end"
            class="dates dates-age"
            data-testid="tree-node-age"
        >
            {{ ageLabel }}
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

/* Direct relation of the hovered node: thicker blue-tinted stroke so the
 * "this person is connected to who you're pointing at" reads without
 * stealing the hovered card's own emphasis. */
.tree-node.related rect {
    stroke: rgb(var(--v-theme-primary) / 0.75);
    stroke-width: 2;
}

/* Anything that's not the hovered node + not a direct relation fades
 * out so the relevant subset visually pops. Transition keeps the swap
 * from jarring; opacity-only animation avoids the SVG-transform pitfall
 * documented in the keyframes comment below. */
.tree-node.dimmed {
    opacity: 0.4;
    transition: opacity 150ms ease-in-out;
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

/* Age cell shares the birth row but sits on the right margin, in a
 * slightly tabular weight so it reads as a stat next to the date. */
.dates-age {
    font:
        600 11px system-ui,
        sans-serif;
    fill: rgb(var(--v-theme-on-surface) / 0.7);
    font-variant-numeric: tabular-nums;
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
