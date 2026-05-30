import { describe, expect, it } from 'vitest'

import { vuetifyDefaults, vuetifyTheme } from '@/design-system/theme'

describe('design-system theme', () => {
    it('declares both Slothlike themes with primary set to the sloth-orange accent', () => {
        expect(vuetifyTheme.defaultTheme).toBe('slothlikeLight')
        expect(vuetifyTheme.themes.slothlikeLight.dark).toBe(false)
        expect(vuetifyTheme.themes.slothlikeDark.dark).toBe(true)
        expect(vuetifyTheme.themes.slothlikeLight.colors?.primary).toBe('#F26A1F')
        expect(vuetifyTheme.themes.slothlikeDark.colors?.primary).toBe('#F26A1F')
    })

    it('declares Slothlike component defaults — pill buttons + outlined fields', () => {
        expect(vuetifyDefaults.VBtn.rounded).toBe('pill')
        expect(vuetifyDefaults.VTextField.variant).toBe('outlined')
        expect(vuetifyDefaults.VCard.rounded).toBe('lg')
        expect(vuetifyDefaults.VChip.rounded).toBe('pill')
        expect(vuetifyDefaults.VAppBar.color).toBe('surface')
    })
})
