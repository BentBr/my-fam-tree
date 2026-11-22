import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn(() => Promise.resolve({ error: undefined })) },
    configureClient: vi.fn(),
}))

import { configureClient, type ClientSessionHooks } from '@/api/client'
import { installApiSession } from '@/plugins/apiSession'
import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'
import type { FamilyId } from '@/types/brand'

import { memoryStorage } from '../helpers/memory-storage'

function installedHooks(navigate = vi.fn()): ClientSessionHooks {
    installApiSession(navigate)
    const hooks = vi.mocked(configureClient).mock.lastCall?.[0]
    if (hooks === undefined) throw new Error('installApiSession did not call configureClient')
    return hooks
}

describe('installApiSession', () => {
    beforeEach(() => {
        vi.stubGlobal('localStorage', memoryStorage())
        vi.stubGlobal('sessionStorage', memoryStorage())
        vi.stubGlobal('navigator', { language: 'en-US' })
        setActivePinia(createPinia())
        vi.mocked(configureClient).mockClear()
    })

    it('reads the family id from the active-family store', () => {
        const hooks = installedHooks()
        expect(hooks.familyId()).toBeNull()
        useActiveFamilyStore().activeFamilyId = 'f-9' as FamilyId
        expect(hooks.familyId()).toBe('f-9')
    })

    it('refreshes through the auth store', async () => {
        const hooks = installedHooks()
        const refresh = vi.spyOn(useAuthStore(), 'refresh').mockResolvedValue()
        await hooks.refreshSession()
        expect(refresh).toHaveBeenCalledTimes(1)
    })

    it('ends an authenticated session: clears claims, logs out, redirects to sign-in', () => {
        const navigate = vi.fn()
        const hooks = installedHooks(navigate)
        const auth = useAuthStore()
        auth.status = 'authenticated'
        const logout = vi.spyOn(auth, 'logout').mockResolvedValue()
        hooks.endSession()
        expect(auth.status).toBe('anonymous')
        expect(logout).toHaveBeenCalledTimes(1)
        expect(navigate).toHaveBeenCalledWith('/auth/sign-in')
    })

    it('does nothing when the session is already anonymous', () => {
        const navigate = vi.fn()
        const hooks = installedHooks(navigate)
        const logout = vi.spyOn(useAuthStore(), 'logout').mockResolvedValue()
        hooks.endSession()
        expect(logout).not.toHaveBeenCalled()
        expect(navigate).not.toHaveBeenCalled()
    })

    it('shows warnings as info toasts', () => {
        const hooks = installedHooks()
        hooks.notifyWarning({ code: 'warning.sibling_partnership', message: 'fallback', path: null }, 'Translated')
        expect(useUiStore().toasts).toEqual([
            expect.objectContaining({ kind: 'info', message: 'Translated', code: 'warning.sibling_partnership' }),
        ])
    })
})
