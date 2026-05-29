import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick, ref } from 'vue'

import { useLocaleStore } from '@/stores/locale'

function mockStorage(): void {
    const store: Record<string, string> = {}
    vi.stubGlobal('localStorage', {
        getItem: (k: string) => store[k] ?? null,
        setItem: (k: string, v: string) => {
            store[k] = v
        },
        removeItem: (k: string) => {
            delete store[k]
        },
    })
}

describe('locale store', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        mockStorage()
    })

    it('defaults to "en" when navigator.language is non-de', () => {
        vi.stubGlobal('navigator', { language: 'en-US' })
        const s = useLocaleStore()
        expect(s.locale).toBe('en')
    })

    it('detects "de" from navigator.language', () => {
        vi.stubGlobal('navigator', { language: 'de-DE' })
        const s = useLocaleStore()
        expect(s.locale).toBe('de')
    })

    it('prefers a stored locale over navigator detection', () => {
        localStorage.setItem('my-fam-tree:locale', 'de')
        vi.stubGlobal('navigator', { language: 'en-US' })
        const s = useLocaleStore()
        expect(s.locale).toBe('de')
    })

    it('ignores an invalid stored value', () => {
        localStorage.setItem('my-fam-tree:locale', 'fr')
        vi.stubGlobal('navigator', { language: 'en-US' })
        const s = useLocaleStore()
        expect(s.locale).toBe('en')
    })

    it('set() updates the ref', () => {
        vi.stubGlobal('navigator', { language: 'en-US' })
        const s = useLocaleStore()
        s.set('de')
        expect(s.locale).toBe('de')
    })

    it('bindToI18n writes initial locale and watches for changes', async () => {
        vi.stubGlobal('navigator', { language: 'en-US' })
        const s = useLocaleStore()
        const localeRef = ref<string>('xx')
        s.bindToI18n({ global: { locale: localeRef } })
        expect(localeRef.value).toBe('en')

        s.set('de')
        await nextTick()
        expect(localeRef.value).toBe('de')
        expect(localStorage.getItem('my-fam-tree:locale')).toBe('de')
    })
})
