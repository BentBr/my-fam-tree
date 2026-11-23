<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
    /** Icon-only buttons for narrow toolbars (phones). */
    compact: boolean
    /** Highlight mode is on. */
    highlight: boolean
    /** Compare mode is on. */
    compare: boolean
}>()

const emit = defineEmits<{
    (e: 'toggle-highlight' | 'toggle-compare'): void
}>()

const { t } = useI18n()
</script>

<template>
    <!-- The tree view's two mode toggles; the view owns the state and keeps
         the modes mutually exclusive.
           - Highlight: clicking a person pins their direct-line lineage (lit
             + sticky while panning) instead of opening the detail drawer.
           - Compare: pick two persons and see how they are related.
         Desktop shows labelled buttons, phones icon-only ones. The filled
         (flat/primary) variant signals the active state; text when off. One
         testid per mode resolves to whichever variant is rendered. -->
    <template v-if="compact">
        <v-btn
            icon="waypoints"
            :variant="highlight ? 'flat' : 'text'"
            :color="highlight ? 'primary' : undefined"
            :active="highlight"
            size="default"
            :title="t('tree.highlightMode')"
            :aria-label="t('tree.highlightMode')"
            data-testid="tree-highlight-mode"
            @click="emit('toggle-highlight')"
        />
        <v-btn
            icon="users"
            :variant="compare ? 'flat' : 'text'"
            :color="compare ? 'primary' : undefined"
            :active="compare"
            size="default"
            :title="t('tree.kinship.compareMode')"
            :aria-label="t('tree.kinship.compareMode')"
            data-testid="tree-compare-mode"
            @click="emit('toggle-compare')"
        />
    </template>
    <template v-else>
        <v-btn
            prepend-icon="waypoints"
            :variant="highlight ? 'flat' : 'text'"
            :color="highlight ? 'primary' : undefined"
            :active="highlight"
            :title="t('tree.highlightMode')"
            data-testid="tree-highlight-mode"
            @click="emit('toggle-highlight')"
        >
            {{ t('tree.highlightMode') }}
        </v-btn>
        <v-btn
            prepend-icon="users"
            :variant="compare ? 'flat' : 'text'"
            :color="compare ? 'primary' : undefined"
            :active="compare"
            :title="t('tree.kinship.compareMode')"
            data-testid="tree-compare-mode"
            @click="emit('toggle-compare')"
        >
            {{ t('tree.kinship.compareMode') }}
        </v-btn>
    </template>
</template>
