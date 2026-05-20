import type { ThemeDefinition } from 'vuetify'

import { colorTokens, radii } from './tokens'

const light: ThemeDefinition = {
    dark: false,
    colors: { ...colorTokens.light },
}
const dark: ThemeDefinition = {
    dark: true,
    colors: { ...colorTokens.dark },
}

export const vuetifyTheme = {
    defaultTheme: 'light',
    themes: { light, dark },
}

/** Component-prop defaults so we don't repeat `variant="flat" rounded` everywhere. */
export const vuetifyDefaults = {
    VBtn: {
        variant: 'flat',
        color: 'primary',
        rounded: 'md',
        style: `border-radius: ${radii.md}px`,
    },
    VTextField: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
    },
    VTextarea: {
        variant: 'outlined',
        density: 'comfortable',
    },
    VSelect: {
        variant: 'outlined',
        density: 'comfortable',
    },
    VCard: {
        elevation: 2,
        rounded: 'lg',
    },
    VChip: {
        variant: 'tonal',
        size: 'small',
    },
    VAlert: {
        variant: 'tonal',
        border: 'start',
    },
    VSwitch: {
        inset: true,
        color: 'primary',
    },
}
