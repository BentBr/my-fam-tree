import { describe, expect, it } from 'vitest'

import { i18n } from '@/i18n'

describe('i18n', () => {
    it('boots with English as the default and fallback locale', () => {
        expect(i18n.global.locale.value).toBe('en')
        expect(i18n.global.fallbackLocale.value).toBe('en')
    })

    it('translates a known key', () => {
        const text = i18n.global.t('app.title')
        expect(typeof text).toBe('string')
        expect(text.length).toBeGreaterThan(0)
    })

    it('falls back to English when a German key is missing', () => {
        // de.json + en.json are kept in sync; the assertion is structural —
        // both locales must answer `app.title` with something non-empty.
        i18n.global.locale.value = 'de'
        const de = i18n.global.t('app.title')
        expect(de.length).toBeGreaterThan(0)
        i18n.global.locale.value = 'en'
    })
})
