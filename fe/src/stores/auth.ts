import { defineStore } from 'pinia'
import { ref } from 'vue'

import { client } from '@/api/client'
import { ApiClientError } from '@/api/errors'
import type { ClaimsPayload } from '@/api/types'
import type { FamilyId, UserId } from '@/types/brand'

import { useLocaleStore } from './locale'

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
        const locale: 'en' | 'de' = c.locale === 'de' ? 'de' : 'en'
        user.value = {
            id: c.user_id as UserId,
            email: c.email,
            locale,
            displayName: '',
        }
        families.value = c.families.map((f) => ({
            id: f.id as FamilyId,
            name: f.name,
            role: f.role,
        }))
        status.value = 'authenticated'
        // Mirror the backend's stored locale into the local store so the i18n
        // switch follows the authenticated identity (overriding the anonymous
        // navigator/localStorage default that detectInitialLocale picked).
        useLocaleStore().set(locale)
    }

    function patchUser(patch: Partial<Pick<AuthUser, 'displayName' | 'locale' | 'email'>>): void {
        if (user.value === null) return
        user.value = { ...user.value, ...patch }
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
        } catch {
            // Backend revocation can fail (e.g., token already revoked, or the
            // network is gone). Either way, still clear local state so the UI
            // doesn't leak the previous session.
        }
        applyClaimsPayload(null)
        // Wipe app-owned local + session storage. All my-fam-tree keys share the
        // `my-fam-tree:` namespace (locale, activeFamily, sidebar, …). HttpOnly
        // cookies can't be cleared from JS — the `Set-Cookie max-age=0` from
        // /auth/logout takes care of that; if that request failed the access
        // cookie may linger until its JWT expires.
        try {
            for (let i = localStorage.length - 1; i >= 0; i--) {
                const key = localStorage.key(i)
                if (key !== null && key.startsWith('my-fam-tree:')) {
                    localStorage.removeItem(key)
                }
            }
            for (let i = sessionStorage.length - 1; i >= 0; i--) {
                const key = sessionStorage.key(i)
                if (key !== null && key.startsWith('my-fam-tree:')) {
                    sessionStorage.removeItem(key)
                }
            }
        } catch {
            // sessionStorage may be unavailable (Safari private mode, etc.).
        }
    }

    return { user, families, status, applyClaimsPayload, patchUser, hydrate, refresh, logout }
})
