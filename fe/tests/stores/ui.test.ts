import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { useUiStore } from '@/stores/ui'

describe('ui store', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        const store: Record<string, string> = {}
        vi.stubGlobal('localStorage', {
            getItem: (k: string) => store[k] ?? null,
            setItem: (k: string, v: string) => {
                store[k] = v
            },
            removeItem: (k: string) => {
                delete store[k]
            },
            clear: () => {
                for (const k of Object.keys(store)) delete store[k]
            },
            key: (i: number) => Object.keys(store)[i] ?? null,
            get length() {
                return Object.keys(store).length
            },
        })
    })

    it('starts with sidebar expanded when no flag is stored', () => {
        const ui = useUiStore()
        expect(ui.sidebarCollapsed).toBe(false)
    })

    it('reads persisted sidebar state from localStorage', () => {
        localStorage.setItem('my-family:sidebar', '1')
        const ui = useUiStore()
        expect(ui.sidebarCollapsed).toBe(true)
    })

    it('toggleSidebar flips state and persists', () => {
        const ui = useUiStore()
        ui.toggleSidebar()
        expect(ui.sidebarCollapsed).toBe(true)
        expect(localStorage.getItem('my-family:sidebar')).toBe('1')
        ui.toggleSidebar()
        expect(ui.sidebarCollapsed).toBe(false)
        expect(localStorage.getItem('my-family:sidebar')).toBe('0')
    })

    it('pushToast appends with a generated id', () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'success', message: 'hi' })
        expect(ui.toasts).toHaveLength(1)
        expect(ui.toasts[0]?.message).toBe('hi')
        expect(ui.toasts[0]?.id).toMatch(/^toast-\d+$/)
    })

    it('pushToast preserves code + requestId when provided', () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'error', message: 'fail', code: 'x' })
        expect(ui.toasts[0]?.code).toBe('x')
    })

    it('dismissToast removes by id', () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'info', message: 'a' })
        ui.pushToast({ kind: 'info', message: 'b' })
        const firstId = ui.toasts[0]?.id ?? ''
        ui.dismissToast(firstId)
        expect(ui.toasts).toHaveLength(1)
        expect(ui.toasts[0]?.message).toBe('b')
    })
})
