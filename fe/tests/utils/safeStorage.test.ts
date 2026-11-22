import { afterEach, describe, expect, it, vi } from 'vitest'

import { safeLocal, safeSession } from '@/utils/safeStorage'

afterEach(() => {
    vi.unstubAllGlobals()
})

describe('safeLocal / safeSession', () => {
    it('reads, writes and removes through a working store', () => {
        const map = new Map<string, string>()
        vi.stubGlobal('localStorage', {
            getItem: (k: string) => map.get(k) ?? null,
            setItem: (k: string, v: string) => void map.set(k, v),
            removeItem: (k: string) => void map.delete(k),
        })
        expect(safeLocal.get('x')).toBeNull()
        safeLocal.set('x', '1')
        expect(safeLocal.get('x')).toBe('1')
        safeLocal.remove('x')
        expect(safeLocal.get('x')).toBeNull()
    })

    it('returns null and no-ops when getItem/setItem throw (private mode)', () => {
        vi.stubGlobal('localStorage', {
            getItem: () => {
                throw new Error('SecurityError')
            },
            setItem: () => {
                throw new Error('QuotaExceededError')
            },
            removeItem: () => {
                throw new Error('SecurityError')
            },
        })
        expect(safeLocal.get('x')).toBeNull()
        expect(() => {
            safeLocal.set('x', '1')
        }).not.toThrow()
        expect(() => {
            safeLocal.remove('x')
        }).not.toThrow()
    })

    it('survives storage being entirely absent', () => {
        // Some embedded webviews throw on the property ACCESS itself.
        vi.stubGlobal('sessionStorage', undefined)
        expect(safeSession.get('x')).toBeNull()
        expect(() => {
            safeSession.set('x', '1')
        }).not.toThrow()
        expect(() => {
            safeSession.remove('x')
        }).not.toThrow()
    })
})
