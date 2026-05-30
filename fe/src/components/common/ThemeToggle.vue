<script setup lang="ts">
/**
 * Theme switcher — a single toggle button, no menu.
 *
 * Click flips the persisted theme between explicit `'light'` and
 * `'dark'` based on whatever is currently resolved (so a first click
 * from `'system'` lands on the opposite of the OS preference). The
 * `'system'` choice still exists in the store — that's the default
 * before any click — but the UI doesn't surface it as a third
 * option. Pre-click, the icon mirrors the OS.
 *
 * Always mounted in the AppBar; visible on every route, including
 * sign-in, public pages, and the consume / invite-accept views.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { currentResolvedTheme } from '@/composables/useThemeMode'
import { useUiStore } from '@/stores/ui'

const ui = useUiStore()
const { t } = useI18n()

const resolved = computed(() => currentResolvedTheme(ui.theme))
// Icon: when the page is currently light, show the moon (= "click to
// go dark"); when dark, show the sun. Mirrors the rdatacore/handoff
// affordance pattern.
const icon = computed(() => (resolved.value === 'dark' ? 'sun' : 'moon'))
const label = computed(() =>
    resolved.value === 'dark' ? t('chrome.theme.switchToLight') : t('chrome.theme.switchToDark'),
)

function toggle(): void {
    ui.setTheme(resolved.value === 'dark' ? 'light' : 'dark')
}
</script>

<template>
    <v-btn
        icon
        variant="text"
        size="small"
        :aria-label="label"
        :title="label"
        data-testid="theme-toggle"
        @click="toggle"
    >
        <v-icon :icon="icon" size="20" />
    </v-btn>
</template>
