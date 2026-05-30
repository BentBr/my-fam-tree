/**
 * Vuetify theme — Slothlike.
 *
 * The colour values below mirror `tokens.css` 1:1; the design-system
 * doc (`.claude/plans/public-site-and-prerender.md`) calls out the
 * mapping. Vuetify consumes a JS object so we re-state the hexes here
 * — keep them in sync if you ever change one in the CSS. Authentication
 * UI, public chrome, and the tree share this single theme (no
 * per-surface overrides).
 *
 * Component defaults live in `slothlikeDefaults` and are picked up via
 * `createVuetify({ defaults })` in `main.ts`. Anything that needs a
 * non-default look (icon-only avatar buttons, etc.) sets it on the
 * instance — but the default is the design system's default, not
 * Vuetify's stock blue.
 */

import type { ThemeDefinition } from 'vuetify'

const slothlikeLight: ThemeDefinition = {
    dark: false,
    colors: {
        background: '#FBF8F4',
        surface: '#FFFFFF',
        'surface-bright': '#FFFFFF',
        'surface-light': '#F6F1EB',
        'surface-variant': '#EFE8DF',
        'on-surface-variant': '#5C5247',
        primary: '#F26A1F',
        'primary-darken-1': '#DD5A12',
        secondary: '#7A4A2B',
        'secondary-darken-1': '#5E3820',
        error: '#C4452F',
        info: '#2E6F9E',
        success: '#2E7D5B',
        warning: '#C2622D',
        'on-background': '#241E18',
        'on-surface': '#241E18',
        'on-primary': '#FFFFFF',
        'on-secondary': '#FFFFFF',
    },
    variables: {
        'border-color': '#241E18',
        'border-opacity': 0.1,
        'high-emphasis-opacity': 0.92,
        'medium-emphasis-opacity': 0.66,
        'disabled-opacity': 0.38,
    },
}

const slothlikeDark: ThemeDefinition = {
    dark: true,
    colors: {
        background: '#100D0A',
        surface: '#1A1511',
        'surface-bright': '#2A231C',
        'surface-light': '#221C16',
        'surface-variant': '#2A231C',
        'on-surface-variant': '#BAAC9E',
        primary: '#F26A1F',
        'primary-darken-1': '#DD5A12',
        secondary: '#C99A6F',
        'secondary-darken-1': '#A87B50',
        error: '#E66A53',
        info: '#5B9BC9',
        success: '#4FB088',
        warning: '#E0894A',
        'on-background': '#F4ECE3',
        'on-surface': '#F4ECE3',
        'on-primary': '#1A0E05',
        'on-secondary': '#1A0E05',
    },
    variables: {
        'border-color': '#FFFFFF',
        'border-opacity': 0.12,
        'high-emphasis-opacity': 0.94,
        'medium-emphasis-opacity': 0.7,
        'disabled-opacity': 0.4,
    },
}

export const vuetifyTheme = {
    /**
     * Default before `useThemeMode` has run — should match `:root` in
     * `tokens.css`. The composable rewrites this immediately on mount
     * to the user's persisted choice (or the system preference).
     */
    defaultTheme: 'slothlikeLight',
    themes: {
        slothlikeLight,
        slothlikeDark,
    },
}

/**
 * Component-prop defaults from the handoff's `theme.ts`. Avoid per-
 * instance overrides — the default is the design system's default.
 */
export const vuetifyDefaults = {
    global: { rounded: 'lg' },
    VBtn: { rounded: 'pill', fontWeight: '700', height: 44, class: 'text-none', elevation: 0 },
    VCard: { rounded: 'lg', elevation: 1, border: true },
    VSheet: { rounded: 'lg' },
    VTextField: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        rounded: 'md',
        hideDetails: 'auto',
    },
    VSelect: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        rounded: 'md',
        hideDetails: 'auto',
    },
    VTextarea: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        rounded: 'md',
    },
    VChip: { rounded: 'pill', size: 'small', label: false },
    VSwitch: { color: 'primary', inset: true, hideDetails: 'auto', density: 'comfortable' },
    VSlider: {
        color: 'primary',
        thumbColor: 'primary',
        trackColor: 'surface-variant',
        hideDetails: 'auto',
    },
    VDialog: { transition: 'dialog-bottom-transition' },
    VList: { rounded: 'md', density: 'comfortable' },
    VAvatar: { rounded: 'circle' },
    VAppBar: { flat: true, color: 'surface', height: 64, border: 'b' },
    VNavigationDrawer: { color: 'surface', border: 'e' },
    VDataTable: { hover: true },
    VTooltip: { location: 'top' },
}
