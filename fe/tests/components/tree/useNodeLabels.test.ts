import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

import type { Positioned } from '@/components/tree/layout'
import { useNodeLabels } from '@/components/tree/useNodeLabels'

function node(over: Partial<Positioned> = {}): Positioned {
    return {
        id: 'n1',
        given_name: 'Alice',
        family_name: 'Smith',
        birth_date: '1990-06-15',
        death_date: null,
        linked_user_id: null,
        is_favourite_for_me: false,
        photo_url: null,
        x: 0,
        y: 0,
        ...over,
    }
}

describe('useNodeLabels', () => {
    beforeEach(() => {
        vi.useFakeTimers()
        vi.setSystemTime(new Date(2026, 5, 1)) // 2026-06-01 local
    })
    afterEach(() => {
        vi.useRealTimers()
    })

    it('builds name, birth label, initials and living age', () => {
        const l = useNodeLabels(node())
        expect(l.fullName.value).toBe('Alice Smith')
        expect(l.birthLabel.value).toBe('* 1990-06-15')
        expect(l.deathLabel.value).toBe('')
        expect(l.hasDates.value).toBe(true)
        expect(l.initials.value).toBe('AS')
        // Birthday (June 15) not reached yet on June 1.
        expect(l.ageLabel.value).toBe('35')
    })

    it('marks age at death and accepts partial ISO dates', () => {
        const l = useNodeLabels(node({ birth_date: '1900', death_date: '1950-03' }))
        expect(l.deathLabel.value).toBe('† 1950-03')
        expect(l.ageLabel.value).toBe('50 (†)')
    })

    it('returns empty labels when dates are missing or unparseable', () => {
        const none = useNodeLabels(node({ birth_date: null }))
        expect(none.birthLabel.value).toBe('')
        expect(none.hasDates.value).toBe(false)
        expect(none.ageLabel.value).toBe('')

        const garbage = useNodeLabels(node({ birth_date: 'unknown' }))
        expect(garbage.ageLabel.value).toBe('')
    })

    it('hides a negative age (death before birth)', () => {
        const l = useNodeLabels(node({ birth_date: '2000-01-01', death_date: '1990-01-01' }))
        expect(l.ageLabel.value).toBe('')
    })

    it('ellipsizes an overlong name and falls back to "?" initials', () => {
        const long = useNodeLabels(node({ given_name: 'Maximilian-Alexander', family_name: 'Wolfeschlegelstein' }))
        expect(long.fullName.value.endsWith('…')).toBe(true)
        expect(long.fullName.value.length).toBeLessThan('Maximilian-Alexander Wolfeschlegelstein'.length)

        const blank = useNodeLabels(node({ given_name: '', family_name: '' }))
        expect(blank.initials.value).toBe('?')
    })

    it('reacts to a changing source', () => {
        const src = ref(node())
        const l = useNodeLabels(src)
        src.value = node({ given_name: 'Bob', family_name: 'Jones' })
        expect(l.fullName.value).toBe('Bob Jones')
        expect(l.initials.value).toBe('BJ')
    })
})
