import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface Toast {
    id: string
    kind: 'info' | 'success' | 'error'
    message: string
    code?: string
    requestId?: string
}

// Monotonic counter for toast IDs. `crypto.randomUUID()` is gated behind a
// secure context (HTTPS / localhost / 127.0.0.1) and throws on plain-HTTP
// dev domains like `http://my-family.docker`. Toast IDs only need to be
// unique inside the array, so a counter is strictly better here.
let toastIdSeq = 0

export const useUiStore = defineStore('ui', () => {
    const sidebarCollapsed = ref(localStorage.getItem('my-family:sidebar') === '1')
    const toasts = ref<Toast[]>([])

    function toggleSidebar(): void {
        sidebarCollapsed.value = !sidebarCollapsed.value
        localStorage.setItem('my-family:sidebar', sidebarCollapsed.value ? '1' : '0')
    }

    function pushToast(t: Omit<Toast, 'id'>): void {
        toastIdSeq += 1
        toasts.value.push({ ...t, id: `toast-${toastIdSeq}` })
    }

    function dismissToast(id: string): void {
        toasts.value = toasts.value.filter((t) => t.id !== id)
    }

    return { sidebarCollapsed, toasts, toggleSidebar, pushToast, dismissToast }
})
