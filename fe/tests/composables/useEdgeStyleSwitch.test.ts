import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h } from 'vue'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn() } }))
const patchSpy = vi.fn()
vi.mock('@/api/hooks/users', () => ({
    useUpdateMe: () => ({ mutate: patchSpy }),
}))

import { useEdgeStyleSwitch } from '@/composables/useEdgeStyleSwitch'
import { useAuthStore } from '@/stores/auth'
import { useUiStore, type EdgeStyle } from '@/stores/ui'
import { memoryStorage } from '../helpers/memory-storage'

/** Mount a throwaway host that calls the composable inside `setup`. */
function mountSwitch(): (next: EdgeStyle) => void {
    let pick!: (next: EdgeStyle) => void
    const Host = defineComponent({
        setup() {
            pick = useEdgeStyleSwitch()
            return () => h('div')
        },
    })
    mount(Host)
    return pick
}

describe('useEdgeStyleSwitch', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('localStorage', memoryStorage())
        vi.stubGlobal('navigator', { language: 'en-US' })
        patchSpy.mockReset()
    })

    it('flips the store and persists to localStorage', () => {
        const pick = mountSwitch()
        pick('angular')
        expect(useUiStore().edgeStyle).toBe('angular')
        expect(localStorage.getItem('my-fam-tree:edge-style')).toBe('angular')
    })

    it('writes the preference to /users/me only when authenticated', () => {
        const pick = mountSwitch()
        pick('angular')
        expect(patchSpy).not.toHaveBeenCalled()

        const auth = useAuthStore()
        auth.applyClaimsPayload({ user_id: 'u', email: 'a@b', locale: 'en', edge_style: 'angular', families: [] })
        pick('curved')
        expect(patchSpy).toHaveBeenCalledWith({ edge_style: 'curved' })
        // Optimistic mirror on the in-memory account.
        expect(auth.user?.edgeStyle).toBe('curved')
    })

    it('is a no-op when picking the already-active style', () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({ user_id: 'u', email: 'a@b', locale: 'en', edge_style: 'curved', families: [] })
        const pick = mountSwitch()
        pick('curved')
        expect(patchSpy).not.toHaveBeenCalled()
        expect(useUiStore().edgeStyle).toBe('curved')
    })
})
