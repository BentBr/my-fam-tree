/**
 * Theme mode composable.
 *
 * Single owner of the side effects that follow from the user's
 * `theme` choice in `useUiStore`:
 *
 *   - Writes `<html data-theme="light|dark">`, which `tokens.css` keys
 *     all colour custom properties off.
 *   - Mirrors the same value onto Vuetify's `theme.global.name` so
 *     component-level dark mode (e.g. `v-card` surface colours) lines
 *     up with the CSS variables.
 *   - When the user picks `'system'`, listens to
 *     `prefers-color-scheme` and re-applies on OS-level theme changes.
 *
 * The composable is *idempotent*: calling `apply()` multiple times with
 * the same effective theme is a no-op. Mount it once at the app root
 * (`App.vue`); other components should read `ui.theme` directly rather
 * than re-invoking this.
 */

import { onBeforeUnmount, watchEffect } from 'vue'
import { useTheme } from 'vuetify'

import { useUiStore, type ThemeMode } from '@/stores/ui'

type ResolvedTheme = 'light' | 'dark'

const MEDIA_QUERY = '(prefers-color-scheme: dark)'
const VUETIFY_LIGHT = 'slothlikeLight'
const VUETIFY_DARK = 'slothlikeDark'

function resolveSystem(): ResolvedTheme {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
        return 'light'
    }
    return window.matchMedia(MEDIA_QUERY).matches ? 'dark' : 'light'
}

function resolveMode(mode: ThemeMode): ResolvedTheme {
    return mode === 'system' ? resolveSystem() : mode
}

/**
 * Wire the store's `theme` ref to the DOM + Vuetify. Returns nothing —
 * the side effects live in the `<html>` attribute and the Vuetify
 * theme. Components that need the resolved value (e.g. to swap an icon
 * between sun/moon) can read `ui.theme` and call `resolveMode()` from
 * this module if needed.
 */
export function useThemeMode(): void {
    const ui = useUiStore()
    const vuetifyTheme = useTheme()

    function apply(): void {
        const resolved = resolveMode(ui.theme)
        const html = document.documentElement
        if (html.getAttribute('data-theme') !== resolved) {
            html.setAttribute('data-theme', resolved)
        }
        const wantedVuetify = resolved === 'dark' ? VUETIFY_DARK : VUETIFY_LIGHT
        if (vuetifyTheme.global.name.value !== wantedVuetify) {
            vuetifyTheme.global.name.value = wantedVuetify
        }
    }

    // Re-apply whenever the persisted choice changes (user clicked
    // the toggle, or another tab wrote localStorage). `watchEffect`
    // also covers the initial mount call.
    watchEffect(apply)

    // When mode is `'system'`, follow the OS preference live. Listener
    // is torn down on unmount; we never leak it.
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return
    const mq = window.matchMedia(MEDIA_QUERY)
    function onSystemChange(): void {
        if (ui.theme === 'system') apply()
    }
    mq.addEventListener('change', onSystemChange)
    onBeforeUnmount(() => {
        mq.removeEventListener('change', onSystemChange)
    })
}

/** Exported so the toggle atom can render a sun-or-moon icon. */
export function currentResolvedTheme(mode: ThemeMode): ResolvedTheme {
    return resolveMode(mode)
}
