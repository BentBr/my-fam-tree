import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { WritableComputedRef } from 'vue'

const STORAGE_KEY = 'my-fam-tree:locale'
export type SupportedLocale = 'en' | 'de'

// Structural shape covering both the I18n root and the Composer it exposes via
// `.global` in non-legacy mode. Typing what we actually touch (a writable
// `locale` ref) avoids invariance pitfalls with vue-i18n 11's deeply-generic
// `I18n` / `Composer` interfaces and keeps us off `any` / `@ts-expect-error`.
interface I18nLike {
    global: {
        locale: WritableComputedRef<string> | { value: string }
    }
}

function detectInitialLocale(): SupportedLocale {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'en' || stored === 'de') return stored
    const nav = navigator.language.toLowerCase()
    if (nav.startsWith('de')) return 'de'
    return 'en'
}

export const useLocaleStore = defineStore('locale', () => {
    const locale = ref<SupportedLocale>(detectInitialLocale())

    function bindToI18n(i18n: I18nLike): void {
        i18n.global.locale.value = locale.value
        watch(locale, (v) => {
            i18n.global.locale.value = v
            localStorage.setItem(STORAGE_KEY, v)
        })
    }

    function set(next: SupportedLocale): void {
        locale.value = next
    }

    return { locale, bindToI18n, set }
})
