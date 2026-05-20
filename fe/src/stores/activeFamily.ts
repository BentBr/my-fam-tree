import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'

import type { FamilyId } from '@/types/brand'

import { useAuthStore, type FamilyMembership, type Role } from './auth'

const STORAGE_KEY = 'my-family:activeFamily'

export const useActiveFamilyStore = defineStore('activeFamily', () => {
    const stored = localStorage.getItem(STORAGE_KEY)
    const activeFamilyId = ref<FamilyId | null>(stored === null ? null : (stored as FamilyId))

    watch(activeFamilyId, (val) => {
        if (val === null) localStorage.removeItem(STORAGE_KEY)
        else localStorage.setItem(STORAGE_KEY, val)
    })

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
    }

    return { activeFamilyId, activeFamily, activeRole, setActive, pickFirstAvailable, clearOnLogout }
})
