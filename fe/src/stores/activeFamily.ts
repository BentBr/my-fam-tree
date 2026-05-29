import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'

import type { FamilyId } from '@/types/brand'

import { useAuthStore, type FamilyMembership, type Role } from './auth'

const STORAGE_KEY = 'my-fam-tree:activeFamily'
const FOCUSED_PERSON_KEY = 'my-fam-tree:focusedPerson'

export const useActiveFamilyStore = defineStore('activeFamily', () => {
    const stored = localStorage.getItem(STORAGE_KEY)
    const activeFamilyId = ref<FamilyId | null>(stored === null ? null : (stored as FamilyId))

    watch(activeFamilyId, (val) => {
        if (val === null) localStorage.removeItem(STORAGE_KEY)
        else localStorage.setItem(STORAGE_KEY, val)
    })

    // Persisted "the person the user last centered on" — survives reloads so
    // the tree re-renders with the same focal point. Wiped by auth.logout()
    // along with the rest of the `my-fam-tree:` namespace.
    const storedFocused = localStorage.getItem(FOCUSED_PERSON_KEY)
    const focusedPersonId = ref<string | null>(storedFocused)

    watch(focusedPersonId, (val) => {
        if (val === null) localStorage.removeItem(FOCUSED_PERSON_KEY)
        else localStorage.setItem(FOCUSED_PERSON_KEY, val)
    })

    function setFocusedPerson(id: string | null): void {
        focusedPersonId.value = id
    }

    const activeFamily = computed<FamilyMembership | null>(() => {
        const auth = useAuthStore()
        const id = activeFamilyId.value
        if (id === null) return null
        return auth.families.find((f) => f.id === id) ?? null
    })

    const activeRole = computed<Role | null>(() => activeFamily.value?.role ?? null)

    function setActive(id: FamilyId): void {
        const auth = useAuthStore()
        if (!auth.families.some((f) => f.id === id)) {
            throw new Error(`not a member of family ${id}`)
        }
        activeFamilyId.value = id
    }

    function pickFirstAvailable(): void {
        const auth = useAuthStore()
        activeFamilyId.value = auth.families[0]?.id ?? null
    }

    function clearOnLogout(): void {
        activeFamilyId.value = null
        focusedPersonId.value = null
    }

    return {
        activeFamilyId,
        activeFamily,
        activeRole,
        focusedPersonId,
        setActive,
        pickFirstAvailable,
        clearOnLogout,
        setFocusedPerson,
    }
})
