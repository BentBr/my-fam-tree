import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { I18n } from 'vue-i18n'

const STORAGE_KEY = 'my-family:locale'
export type SupportedLocale = 'en' | 'de'

function detectInitialLocale(): SupportedLocale {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'en' || stored === 'de') return stored
    const nav = navigator.language.toLowerCase()
    if (nav.startsWith('de')) return 'de'
    return 'en'
}

export const useLocaleStore = defineStore('locale', () => {
    const locale = ref<SupportedLocale>(detectInitialLocale())

    function bindToI18n(i18n: I18n<unknown, unknown, unknown, string, false>): void {
        i18n.global.locale = locale.value
        watch(locale, (v) => {
            i18n.global.locale = v
            localStorage.setItem(STORAGE_KEY, v)
        })
    }

    function set(next: SupportedLocale): void {
        locale.value = next
    }

    return { locale, bindToI18n, set }
})
