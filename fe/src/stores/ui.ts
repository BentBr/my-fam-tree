import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface Toast {
    id: string
    kind: 'info' | 'success' | 'error'
    message: string
    code?: string
    requestId?: string
}

/**
 * Theme mode persisted to localStorage. `'system'` follows the OS via
 * `prefers-color-scheme`; `'light'` / `'dark'` are explicit overrides
 * a user has clicked. Resolved-to-light-or-dark logic + the
 * `<html data-theme>` write live in `useThemeMode`.
 */
export type ThemeMode = 'system' | 'light' | 'dark'

const THEME_KEY = 'my-fam-tree:theme'

function loadInitialTheme(): ThemeMode {
    const v = localStorage.getItem(THEME_KEY)
    return v === 'light' || v === 'dark' || v === 'system' ? v : 'system'
}

// Monotonic counter for toast IDs. `crypto.randomUUID()` is gated behind a
// secure context (HTTPS / localhost / 127.0.0.1) and throws on plain-HTTP
// dev domains like `http://my-fam-tree.docker`. Toast IDs only need to be
// unique inside the array, so a counter is strictly better here.
let toastIdSeq = 0

export const useUiStore = defineStore('ui', () => {
    const sidebarCollapsed = ref(localStorage.getItem('my-fam-tree:sidebar') === '1')
    const toasts = ref<Toast[]>([])
    const theme = ref<ThemeMode>(loadInitialTheme())

    function toggleSidebar(): void {
        sidebarCollapsed.value = !sidebarCollapsed.value
        localStorage.setItem('my-fam-tree:sidebar', sidebarCollapsed.value ? '1' : '0')
    }

    function setTheme(next: ThemeMode): void {
        theme.value = next
        localStorage.setItem(THEME_KEY, next)
    }

    function pushToast(t: Omit<Toast, 'id'>): void {
        // Dedupe: a burst of N concurrent fetches that all 401 (or otherwise
        // fail with the same `Validation` violation) would otherwise stack
        // N identical toasts. Two toasts are "the same" when their kind,
        // message, and code (when set) all match — different texts or
        // different codes still surface independently. We compare against
        // toasts currently in the stack; once one is dismissed (manually or
        // by auto-timeout), a fresh failure can surface again.
        const duplicate = toasts.value.some(
            (existing) =>
                existing.kind === t.kind && existing.message === t.message && (existing.code ?? '') === (t.code ?? ''),
        )
        if (duplicate) return
        toastIdSeq += 1
        toasts.value.push({ ...t, id: `toast-${toastIdSeq}` })
    }

    function dismissToast(id: string): void {
        toasts.value = toasts.value.filter((t) => t.id !== id)
    }

    return { sidebarCollapsed, toasts, theme, toggleSidebar, setTheme, pushToast, dismissToast }
})
