import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { useUiStore } from '@/stores/ui'
import { memoryStorage } from '../helpers/memory-storage'

describe('ui store', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('localStorage', memoryStorage())
    })

    it('starts with sidebar expanded when no flag is stored', () => {
        const ui = useUiStore()
        expect(ui.sidebarCollapsed).toBe(false)
    })

    it('reads persisted sidebar state from localStorage', () => {
        localStorage.setItem('my-fam-tree:sidebar', '1')
        const ui = useUiStore()
        expect(ui.sidebarCollapsed).toBe(true)
    })

    it('toggleSidebar flips state and persists', () => {
        const ui = useUiStore()
        ui.toggleSidebar()
        expect(ui.sidebarCollapsed).toBe(true)
        expect(localStorage.getItem('my-fam-tree:sidebar')).toBe('1')
        ui.toggleSidebar()
        expect(ui.sidebarCollapsed).toBe(false)
        expect(localStorage.getItem('my-fam-tree:sidebar')).toBe('0')
    })

    it('defaults to curved tree lines when nothing is stored', () => {
        expect(useUiStore().edgeStyle).toBe('curved')
    })

    it('reads a persisted angular edge style from localStorage', () => {
        localStorage.setItem('my-fam-tree:edge-style', 'angular')
        expect(useUiStore().edgeStyle).toBe('angular')
    })

    it('falls back to curved for an unknown stored edge style', () => {
        localStorage.setItem('my-fam-tree:edge-style', 'zigzag')
        expect(useUiStore().edgeStyle).toBe('curved')
    })

    it('setEdgeStyle updates state and persists', () => {
        const ui = useUiStore()
        ui.setEdgeStyle('angular')
        expect(ui.edgeStyle).toBe('angular')
        expect(localStorage.getItem('my-fam-tree:edge-style')).toBe('angular')
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

    it('pushToast deduplicates identical (kind+message+code) entries already on screen', () => {
        const ui = useUiStore()
        // The "burst of session_expired" repro: four concurrent 401 toasts
        // collapse to one.
        for (let i = 0; i < 4; i += 1) {
            ui.pushToast({
                kind: 'error',
                message: 'Your session expired — please sign in again.',
                code: 'session_expired',
            })
        }
        expect(ui.toasts).toHaveLength(1)
    })

    it('pushToast still surfaces a different message even when kind + code match', () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'error', message: 'A', code: 'validation' })
        ui.pushToast({ kind: 'error', message: 'B', code: 'validation' })
        expect(ui.toasts).toHaveLength(2)
    })

    it('pushToast lets the same message reappear once the previous toast is dismissed', () => {
        const ui = useUiStore()
        ui.pushToast({ kind: 'error', message: 'hi' })
        ui.pushToast({ kind: 'error', message: 'hi' })
        expect(ui.toasts).toHaveLength(1)
        ui.dismissToast(ui.toasts[0]?.id ?? '')
        ui.pushToast({ kind: 'error', message: 'hi' })
        expect(ui.toasts).toHaveLength(1)
    })
})
