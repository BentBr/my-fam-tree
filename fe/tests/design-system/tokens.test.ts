import { describe, expect, it } from 'vitest'

import { AVATAR_TONES, TREE_TOKENS, avatarTone, radii } from '@/design-system/tokens'

describe('design-system tokens', () => {
    it('exposes the four avatar tone variants with hex bg + fg', () => {
        for (const key of ['female', 'male', 'diverse', 'deceased'] as const) {
            expect(AVATAR_TONES[key].bg).toMatch(/^#[0-9A-Fa-f]{6}$/)
            expect(AVATAR_TONES[key].fg).toMatch(/^#[0-9A-Fa-f]{6}$/)
        }
    })

    it('avatarTone falls back to the diverse palette for unknown sex', () => {
        expect(avatarTone('unknown', false)).toEqual(AVATAR_TONES.diverse)
    })

    it('avatarTone overrides to deceased grey regardless of sex when deceased=true', () => {
        expect(avatarTone('female', true)).toEqual(AVATAR_TONES.deceased)
        expect(avatarTone('male', true)).toEqual(AVATAR_TONES.deceased)
    })

    it('TREE_TOKENS has light + dark variants for the three connector roles', () => {
        for (const mode of ['light', 'dark'] as const) {
            expect(TREE_TOKENS[mode].edge).toMatch(/^#/)
            expect(TREE_TOKENS[mode].partner).toMatch(/^#/)
            expect(TREE_TOKENS[mode].grid).toMatch(/^#/)
        }
    })

    it('radii are in ascending size order', () => {
        expect(radii.xs).toBeLessThan(radii.sm)
        expect(radii.sm).toBeLessThan(radii.md)
        expect(radii.md).toBeLessThan(radii.lg)
        expect(radii.lg).toBeLessThan(radii.xl)
        expect(radii.pill).toBeGreaterThan(radii.xl)
    })
})
