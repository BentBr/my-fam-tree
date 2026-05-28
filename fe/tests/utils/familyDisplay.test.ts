import { describe, expect, it } from 'vitest'

import { duplicateNameSet, formatFamilyDate } from '@/utils/familyDisplay'

describe('duplicateNameSet', () => {
    it('returns the names that appear more than once, ignoring unique entries', () => {
        const dups = duplicateNameSet([
            { name: 'Müller' },
            { name: 'Peters' },
            { name: 'Peters' },
            { name: 'Schmidt' },
            { name: 'Peters' },
        ])
        expect(dups.has('Peters')).toBe(true)
        expect(dups.has('Müller')).toBe(false)
        expect(dups.has('Schmidt')).toBe(false)
        expect(dups.size).toBe(1)
    })

    it('handles an empty list', () => {
        expect(duplicateNameSet([])).toEqual(new Set())
    })

    it('catches all duplicate names in a many-collision list', () => {
        const dups = duplicateNameSet([{ name: 'A' }, { name: 'A' }, { name: 'B' }, { name: 'B' }, { name: 'C' }])
        expect(dups).toEqual(new Set(['A', 'B']))
    })
})

describe('formatFamilyDate', () => {
    it('renders an English short date for valid ISO timestamps', () => {
        // Format uses `month: 'short'` so 'May 2026' surfaces in English locale.
        expect(formatFamilyDate('2026-05-01T12:34:56Z', 'en')).toMatch(/May.*2026|2026.*May/)
    })

    it('renders a German short date for valid ISO timestamps', () => {
        // German uses 'Mai' for month name. The exact ordering depends on
        // ICU; just assert the month/year are present.
        const out = formatFamilyDate('2026-05-01T12:34:56Z', 'de')
        expect(out).toMatch(/Mai/)
        expect(out).toMatch(/2026/)
    })

    it('returns null for empty / null / undefined input (so callers can fall back)', () => {
        expect(formatFamilyDate(null, 'en')).toBeNull()
        expect(formatFamilyDate(undefined, 'en')).toBeNull()
        expect(formatFamilyDate('', 'en')).toBeNull()
    })

    it('returns null for an unparseable timestamp rather than throwing or rendering NaN', () => {
        expect(formatFamilyDate('not-a-date', 'en')).toBeNull()
    })
})
