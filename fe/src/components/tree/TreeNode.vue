<script setup lang="ts">
import { NODE_H, NODE_W, type Positioned } from './layout'
import TreeNodeFavourite from './TreeNodeFavourite.vue'
import { TEXT_LEFT, TEXT_RIGHT_PAD, useNodeLabels } from './useNodeLabels'

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
    /** Toggle the per-user favourite mark. Parent decides whether to
     * call into `useSetFavourite` — TreeNode stays presentational and
     * never touches the network directly. */
    (e: 'toggle-favourite', id: string, next: boolean): void
}>()

const { fullName, birthLabel, deathLabel, hasDates, ageLabel, initials } = useNodeLabels(() => props.node)

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
        <TreeNodeFavourite
            :node-id="props.node.id"
            :filled="props.node.is_favourite_for_me"
            @toggle="(id, next) => emit('toggle-favourite', id, next)"
        />
        <!--
            Avatar bubble. When the node has a presigned `photo_url` we
            render the image masked by a circular `clipPath` so it stays
            round (the rest of the UI uses round avatars; PersonDetail's
            big squared photo is a deliberate exception). Falling back to
            the original initials circle keeps the empty-photo render the
            same as before.
            The clipPath id is namespaced by the node id so two trees on
            the same page can't collide on the global SVG id space.
        -->
        <template v-if="props.node.photo_url">
            <defs>
                <clipPath :id="`tree-node-avatar-clip-${props.node.id}`">
                    <circle :cx="32" :cy="NODE_H / 2" r="22" />
                </clipPath>
            </defs>
            <image
                :x="10"
                :y="NODE_H / 2 - 22"
                width="44"
                height="44"
                :href="props.node.photo_url"
                :clip-path="`url(#tree-node-avatar-clip-${props.node.id})`"
                preserveAspectRatio="xMidYMid slice"
                :data-testid="`tree-node-photo-${props.node.id}`"
            />
        </template>
        <template v-else>
            <circle :cx="32" :cy="NODE_H / 2" r="22" class="avatar" />
            <text :x="32" :y="NODE_H / 2 + 6" text-anchor="middle" class="initials">
                {{ initials }}
            </text>
        </template>
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

/* Outer card rect. The first <rect> child is the card itself — the
 * later <rect> inside `.favourite` is the transparent hit target for
 * the star and must NOT inherit these styles. We scope via the
 * direct-child selector.
 *
 * NB: avoid `rgb(var(--v-theme-…) / α)` for fills/strokes here. Vuetify
 * emits `--v-theme-*` as comma-separated RGB triplets, so the
 * slash-alpha CSS syntax (which requires SPACE-separated channels) is
 * INVALID and silently drops the declaration — the SVG element then
 * falls back to its default (black for text, none for fills). The
 * cards used to render with near-invisible dates in dark mode for
 * exactly that reason. Use the design-system tokens from
 * `tokens.css` (`--text`, `--text-2`, `--text-3`, `--acc`, etc.) so
 * the values are plain hex and flip light/dark in lock-step. */
.tree-node > rect {
    fill: var(--surface);
    stroke: var(--border-2);
    stroke-width: 1.5;
    transition:
        stroke 150ms ease-in-out,
        stroke-width 150ms ease-in-out;
}
.tree-node.selected > rect,
.tree-node:focus-visible > rect {
    stroke: var(--acc);
    stroke-width: 2;
}

/* Hovered node: stronger accent stroke + an orange-tinted drop shadow
 * that radiates a few pixels past the card. CSS `filter: drop-shadow`
 * on the <g> wrapper is the only SVG-safe way to glow without a
 * <filter> def (we already define one on FamilyTree.vue, but reusing
 * it would burn an extra layer per card; the cheap drop-shadow is fine
 * for the few cards in the related subset). The transition makes the
 * glow build up smoothly when the pointer enters / leaves. */
.tree-node {
    transition: filter 150ms ease-in-out;
}
.tree-node.hovered > rect {
    stroke: var(--acc-strong);
    stroke-width: 3;
}
.tree-node.hovered {
    /* Double drop-shadow: a tight bright core + a wider halo so the
     * focused card reads as the unmistakable centre of attention. */
    filter: drop-shadow(0 0 4px var(--acc-strong)) drop-shadow(0 0 10px var(--acc));
}

/* Lineage relation of the hovered node — same accent stroke family as
 * `.hovered` but one notch softer so the hover-focus card still reads as
 * the centre of attention while the whole origin line stays clearly lit. */
.tree-node.related > rect {
    stroke: var(--acc-strong);
    stroke-width: 2.5;
}
.tree-node.related {
    filter: drop-shadow(0 0 7px var(--acc));
}

/* Anything that's not the hovered node + not a direct relation fades
 * harder so the lit lineage pops. Transition keeps the swap from
 * jarring; opacity-only animation avoids the SVG-transform pitfall
 * documented in the keyframes comment below. */
.tree-node.dimmed {
    opacity: 0.28;
    transition: opacity 150ms ease-in-out;
}

.avatar {
    fill: var(--acc-soft);
    stroke: var(--acc);
    stroke-width: 1;
}
.initials {
    font:
        600 14px system-ui,
        sans-serif;
    fill: var(--acc-strong);
    pointer-events: none;
}
.name {
    font:
        600 13px system-ui,
        sans-serif;
    fill: var(--text);
    pointer-events: none;
}
.dates {
    font:
        11px system-ui,
        sans-serif;
    fill: var(--text-2);
    pointer-events: none;
}
/* Death date sits on its own row below birth and reads smaller + fainter
 * so the "*" / "†" lines pair visually like a small obituary detail. */
.dates-death {
    font:
        9.5px system-ui,
        sans-serif;
    fill: var(--text-3);
}

/* Age cell shares the birth row but sits on the right margin, in a
 * slightly tabular weight so it reads as a stat next to the date. */
.dates-age {
    font:
        600 11px system-ui,
        sans-serif;
    fill: var(--text-2);
    font-variant-numeric: tabular-nums;
}

/* Baseline "this is you" treatment: a star-amber ring + soft acc-soft
 * wash so the user marker reads as an identity badge rather than a
 * transient selection state. Tokens come from the design system so
 * the values flip in dark mode automatically. */
.tree-node.current-user > rect {
    stroke: var(--star);
    stroke-width: 2.5;
    fill: var(--acc-soft);
}
.tree-node.current-user.selected > rect,
.tree-node.current-user:focus-visible > rect {
    stroke-width: 3;
}

/* Deceased cards get a soft grey wash so they read as historical at a
 * glance. The avatar+text inside the SVG can't use `filter: grayscale`
 * (SVG filters require a <filter> def), so we tint the rect fill and
 * the stroke instead, and lighten the text/initials. All four values
 * come from `tokens.css` so the dark-mode variant is automatic. The
 * card joins the lineage glow when hovered or in the focus set via
 * the unmodified `.hovered` / `.related` rules above. */
.tree-node.deceased > rect {
    fill: var(--surface-2);
    stroke: var(--border);
}
.tree-node.deceased .avatar {
    fill: var(--av-dead-bg);
    stroke: var(--border-2);
}
.tree-node.deceased .initials {
    fill: var(--av-dead-fg);
}
.tree-node.deceased .name,
.tree-node.deceased .dates {
    fill: var(--text-3);
}

/* Deceased + hovered/related: the grey wash above is defined LATER than
 * the generic `.hovered`/`.related` rect rules, so without these it
 * would override the accent stroke and a deceased ancestor would stay
 * grey on hover — impossible to follow up the origin line. These rules
 * sit after the wash and are more specific, so they win: the card
 * reclaims the accent stroke AND lifts its text/avatar back to full
 * contrast while hovered or lit as part of the lineage. The historical
 * grey returns the moment the pointer leaves. */
.tree-node.deceased.hovered > rect {
    fill: var(--acc-soft);
    stroke: var(--acc-strong);
    stroke-width: 3;
}
.tree-node.deceased.related > rect {
    fill: var(--acc-soft);
    stroke: var(--acc-strong);
    stroke-width: 2.5;
}
.tree-node.deceased.hovered .name,
.tree-node.deceased.related .name {
    fill: var(--text);
}
.tree-node.deceased.hovered .dates,
.tree-node.deceased.related .dates {
    fill: var(--text-2);
}
.tree-node.deceased.hovered .avatar,
.tree-node.deceased.related .avatar {
    fill: var(--acc-soft);
    stroke: var(--acc);
}
.tree-node.deceased.hovered .initials,
.tree-node.deceased.related .initials {
    fill: var(--acc-strong);
}
</style>
