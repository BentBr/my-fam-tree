import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface Toast {
    id: string
    kind: 'info' | 'success' | 'error'
    message: string
}

export const useUiStore = defineStore('ui', () => {
    const sidebarCollapsed = ref(localStorage.getItem('my-family:sidebar') === '1')
    const toasts = ref<Toast[]>([])

    function toggleSidebar(): void {
        sidebarCollapsed.value = !sidebarCollapsed.value
        localStorage.setItem('my-family:sidebar', sidebarCollapsed.value ? '1' : '0')
    }

    function pushToast(t: Omit<Toast, 'id'>): void {
        toasts.value.push({ ...t, id: crypto.randomUUID() })
    }

    function dismissToast(id: string): void {
        toasts.value = toasts.value.filter((t) => t.id !== id)
    }

    return { sidebarCollapsed, toasts, toggleSidebar, pushToast, dismissToast }
})
