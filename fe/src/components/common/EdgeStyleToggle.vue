<script setup lang="ts">
/**
 * Tree connector-style switcher — a single toggle button, no menu.
 *
 * Flips the persisted `edgeStyle` in `useUiStore` between the smooth
 * `'curved'` default and right-angled `'angular'` lines. Like
 * `ThemeToggle`, the icon and label describe the state a click leads
 * to, not the current one.
 *
 * Mounted in the AppBar on the tree route only; on phones the same
 * switch lives in the `AccountControl` dropdown instead.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { useUiStore } from '@/stores/ui'

const ui = useUiStore()
const { t } = useI18n()

const isAngular = computed(() => ui.edgeStyle === 'angular')
const icon = computed(() => (isAngular.value ? 'spline' : 'network'))
const label = computed(() =>
    isAngular.value ? t('chrome.edgeStyle.switchToCurved') : t('chrome.edgeStyle.switchToAngular'),
)

function toggle(): void {
    ui.setEdgeStyle(isAngular.value ? 'curved' : 'angular')
}
</script>

<template>
    <v-btn
        icon
        variant="text"
        size="small"
        :aria-label="label"
        :title="label"
        data-testid="edge-style-toggle"
        @click="toggle"
    >
        <v-icon :icon="icon" size="20" />
    </v-btn>
</template>
