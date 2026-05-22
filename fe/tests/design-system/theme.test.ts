import { describe, expect, it } from 'vitest'

import { vuetifyDefaults, vuetifyTheme } from '@/design-system/theme'

describe('design-system theme', () => {
    it('declares both light and dark themes with the colour palettes', () => {
        expect(vuetifyTheme.defaultTheme).toBe('light')
        expect(vuetifyTheme.themes.light.dark).toBe(false)
        expect(vuetifyTheme.themes.dark.dark).toBe(true)
        expect(vuetifyTheme.themes.light.colors?.primary).toBeDefined()
    })

    it('declares component defaults for the core form controls', () => {
        expect(vuetifyDefaults.VBtn.variant).toBe('flat')
        expect(vuetifyDefaults.VTextField.variant).toBe('outlined')
        expect(vuetifyDefaults.VCard.elevation).toBe(2)
        expect(vuetifyDefaults.VChip.size).toBe('small')
    })
})
