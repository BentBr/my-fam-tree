<script setup lang="ts">
import { ref } from 'vue'

import { NODE_H, NODE_W, type Positioned } from './layout'

const props = defineProps<{
    node: Positioned
    selected: boolean
}>()

const emit = defineEmits<{
    (e: 'select', id: string): void
}>()

const hovered = ref(false)

function dateLabel(p: Positioned): string {
    const b = p.birth_date ?? ''
    const d = p.death_date ?? ''
    if (b === '' && d === '') return ''
    if (d === '') return b
    return `${b} – ${d}`
}

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
        :class="['tree-node', { selected: props.selected, hovered }]"
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
        <foreignObject :x="64" :y="6" :width="NODE_W - 72" :height="NODE_H - 12">
            <div class="node-body">
                <div class="name">{{ props.node.given_name }} {{ props.node.family_name }}</div>
                <div class="dates">{{ dateLabel(props.node) }}</div>
            </div>
        </foreignObject>
    </g>
</template>

<style scoped>
.tree-node {
    cursor: pointer;
    /* Soft fade-in whenever the node mounts (re-layout after add/remove). */
    animation: tree-node-in 300ms ease-out both;
    transition: transform 200ms ease-out;
}
@keyframes tree-node-in {
    from {
        opacity: 0;
        transform: scale(0.96);
    }
    to {
        opacity: 1;
        transform: scale(1);
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
.node-body {
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    font:
        13px system-ui,
        sans-serif;
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: rgb(var(--v-theme-on-surface));
}
.name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.dates {
    color: rgb(var(--v-theme-on-surface) / 0.6);
    font-size: 11px;
}
</style>
