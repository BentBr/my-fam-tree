import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'

import type { ClaimsPayload } from '@/api/types'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import type { FamilyId } from '@/types/brand'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))
vi.mock('@/router', () => ({ router: { replace: vi.fn() } }))

function mockStorage(): void {
    const store: Record<string, string> = {}
    vi.stubGlobal('localStorage', {
        getItem: (k: string) => store[k] ?? null,
        setItem: (k: string, v: string) => {
            store[k] = v
        },
        removeItem: (k: string) => {
            delete store[k]
        },
        key: (i: number) => Object.keys(store)[i] ?? null,
        get length() {
            return Object.keys(store).length
        },
    })
}

function applyTwoFamilies(): void {
    const auth = useAuthStore()
    auth.applyClaimsPayload({
        user_id: 'u-1',
        email: 'a@b',
        locale: 'en',
        families: [
            { id: 'f-1', name: 'F1', role: 'owner' },
            { id: 'f-2', name: 'F2', role: 'user' },
        ],
    } as ClaimsPayload)
}

describe('activeFamily store', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        mockStorage()
    })

    it('starts with no active family when nothing is stored', () => {
        const family = useActiveFamilyStore()
        expect(family.activeFamilyId).toBeNull()
        expect(family.activeFamily).toBeNull()
        expect(family.activeRole).toBeNull()
    })

    it('reads stored familyId from localStorage on init', () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'f-1')
        const family = useActiveFamilyStore()
        expect(family.activeFamilyId).toBe('f-1')
    })

    it('persists activeFamilyId changes back to localStorage', async () => {
        applyTwoFamilies()
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        await nextTick()
        expect(localStorage.getItem('my-fam-tree:activeFamily')).toBe('f-1')
    })

    it('clears stored value when set to null', async () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'f-1')
        const family = useActiveFamilyStore()
        family.clearOnLogout()
        await nextTick()
        expect(localStorage.getItem('my-fam-tree:activeFamily')).toBeNull()
    })

    it('setActive throws when family is unknown', () => {
        applyTwoFamilies()
        const family = useActiveFamilyStore()
        expect(() => family.setActive('not-a-member' as FamilyId)).toThrow(/not a member/)
    })

    it('activeFamily / activeRole resolve through auth.families', () => {
        applyTwoFamilies()
        const family = useActiveFamilyStore()
        family.setActive('f-2' as FamilyId)
        expect(family.activeFamily?.name).toBe('F2')
        expect(family.activeRole).toBe('user')
    })

    it('activeFamily is null when id no longer matches a membership', () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [],
        } as ClaimsPayload)
        localStorage.setItem('my-fam-tree:activeFamily', 'stale')
        const family = useActiveFamilyStore()
        expect(family.activeFamily).toBeNull()
    })

    it('pickFirstAvailable selects the first family, or null when none', () => {
        applyTwoFamilies()
        const family = useActiveFamilyStore()
        family.pickFirstAvailable()
        expect(family.activeFamilyId).toBe('f-1')

        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [],
        } as ClaimsPayload)
        family.pickFirstAvailable()
        expect(family.activeFamilyId).toBeNull()
    })

    it('setFocusedPerson persists to localStorage; null clears it', async () => {
        const family = useActiveFamilyStore()
        family.setFocusedPerson('p-1')
        await nextTick()
        expect(localStorage.getItem('my-fam-tree:focusedPerson')).toBe('p-1')
        family.setFocusedPerson(null)
        await nextTick()
        expect(localStorage.getItem('my-fam-tree:focusedPerson')).toBeNull()
    })

    it('reads stored focusedPersonId on init', () => {
        localStorage.setItem('my-fam-tree:focusedPerson', 'p-x')
        const family = useActiveFamilyStore()
        expect(family.focusedPersonId).toBe('p-x')
    })

    it('clearOnLogout wipes both fields', () => {
        applyTwoFamilies()
        const family = useActiveFamilyStore()
        family.setActive('f-1' as FamilyId)
        family.setFocusedPerson('p-1')
        family.clearOnLogout()
        expect(family.activeFamilyId).toBeNull()
        expect(family.focusedPersonId).toBeNull()
    })
})
