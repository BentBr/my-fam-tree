import { describe, expect, it } from 'vitest'

import { brand, unsafeCast } from '@/types/brand'

describe('brand', () => {
    it('user() narrows a string to UserId', () => {
        const id = brand.user('user-123')
        expect(id).toBe('user-123')
    })

    it('family() narrows a string to FamilyId', () => {
        const id = brand.family('fam-7')
        expect(id).toBe('fam-7')
    })

    it('person() narrows a string to PersonId', () => {
        const id = brand.person('p-42')
        expect(id).toBe('p-42')
    })

    it('familyMembership() narrows a string to FamilyMembershipId', () => {
        const id = brand.familyMembership('fm-1')
        expect(id).toBe('fm-1')
    })
})

describe('unsafeCast', () => {
    it('returns the input value untouched', () => {
        const value = { foo: 'bar' }
        const out = unsafeCast<{ foo: string }>(value, 'test escape hatch')
        expect(out).toBe(value)
    })

    it('does not throw on null', () => {
        expect(() => unsafeCast<unknown>(null, 'null cast')).not.toThrow()
    })
})
