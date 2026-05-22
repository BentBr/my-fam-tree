import { describe, expect, it } from 'vitest'

import { colorTokens, radii } from '@/design-system/tokens'

describe('design-system tokens', () => {
    it('exposes light + dark colour palettes', () => {
        expect(colorTokens.light.primary).toMatch(/^#/)
        expect(colorTokens.dark.primary).toMatch(/^#/)
    })

    it('exposes radii in ascending size order', () => {
        expect(radii.sm).toBeLessThan(radii.md)
        expect(radii.md).toBeLessThan(radii.lg)
        expect(radii.lg).toBeLessThan(radii.xl)
        expect(radii.pill).toBeGreaterThan(radii.xl)
    })
})
