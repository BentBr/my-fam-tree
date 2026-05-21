import { defineStore } from 'pinia'
import { ref } from 'vue'

import { client } from '@/api/client'
import { ApiClientError } from '@/api/errors'
import type { ClaimsPayload } from '@/api/types'
import type { FamilyId, UserId } from '@/types/brand'

export type Role = 'user' | 'admin' | 'owner'

export type { ClaimsPayload }

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
        try {
            const { data, error } = await client.GET('/api/v1/auth/me')
            if (error !== undefined) throw error
            if (data !== undefined) {
                applyClaimsPayload(data.data)
            }
        } catch (e: unknown) {
            if (e instanceof ApiClientError && e.status === 401) {
                applyClaimsPayload(null)
                return
            }
            throw e
        }
    }

    async function refresh(): Promise<void> {
        const { data, error } = await client.POST('/api/v1/auth/refresh')
        if (error !== undefined) throw error
        if (data !== undefined) {
            applyClaimsPayload(data.data)
        }
    }

    async function logout(): Promise<void> {
        try {
            const { error } = await client.POST('/api/v1/auth/logout')
            if (error !== undefined) throw error
        } finally {
            applyClaimsPayload(null)
        }
    }

    return { user, families, status, applyClaimsPayload, hydrate, refresh, logout }
})
