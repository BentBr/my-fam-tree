import { describe, expect, it } from 'vitest'

import { toLocalIsoDate } from '@/utils/dates'

describe('toLocalIsoDate', () => {
    it('formats a local date as YYYY-MM-DD with zero padding', () => {
        expect(toLocalIsoDate(new Date(2026, 0, 5))).toBe('2026-01-05')
    })

    it('uses local time, not UTC (no day shift near midnight)', () => {
        expect(toLocalIsoDate(new Date(2026, 11, 31, 23, 59))).toBe('2026-12-31')
    })

    it('pads years below 1000 to four digits', () => {
        const d = new Date(2026, 6, 14)
        d.setFullYear(870)
        expect(toLocalIsoDate(d)).toBe('0870-07-14')
    })
})
