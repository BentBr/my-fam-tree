import { defineStore } from 'pinia'
import { ref } from 'vue'

import type { FamilyId, UserId } from '@/types/brand'

export type Role = 'user' | 'admin' | 'owner'

export interface FamilyMembership {
    id: FamilyId
    name: string
    role: Role
}

export interface AuthUser {
    id: UserId
    email: string
    locale: 'en' | 'de'
    displayName: string
}

interface ClaimsPayload {
    user_id: string
    email: string
    locale: string
    families: Array<{ id: string; name: string; role: Role }>
}

export const useAuthStore = defineStore('auth', () => {
    const user = ref<AuthUser | null>(null)
    const families = ref<FamilyMembership[]>([])
    const status = ref<'anonymous' | 'authenticated'>('anonymous')

    function applyClaimsPayload(c: ClaimsPayload | null): void {
        if (c === null) {
            user.value = null
            families.value = []
            status.value = 'anonymous'
            return
        }
        user.value = {
            id: c.user_id as UserId,
            email: c.email,
            locale: (c.locale === 'de' ? 'de' : 'en') as 'en' | 'de',
            displayName: '',
        }
        families.value = c.families.map((f) => ({
            id: f.id as FamilyId,
            name: f.name,
            role: f.role,
        }))
        status.value = 'authenticated'
    }

    async function hydrate(): Promise<void> {
        // Implemented in Phase 1 (calls GET /auth/me). Phase 0d stays anonymous.
    }

    async function refresh(): Promise<void> {
        // Implemented in Phase 1 (calls POST /auth/refresh).
    }

    async function logout(): Promise<void> {
        // Implemented in Phase 1 (calls POST /auth/logout).
        applyClaimsPayload(null)
    }

    return { user, families, status, applyClaimsPayload, hydrate, refresh, logout }
})
