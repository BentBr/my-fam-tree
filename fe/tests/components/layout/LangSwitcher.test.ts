import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn() } }))

import { client } from '@/api/client'
import LangSwitcher from '@/components/layout/LangSwitcher.vue'
import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'

function makeQueryClient(): QueryClient {
    return new QueryClient({ defaultOptions: { queries: { retry: 0 }, mutations: { retry: 0 } } })
}

interface MockedClient {
    PATCH: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

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

function mountSwitcher() {
    return mount(LangSwitcher, {
        global: {
            plugins: [createPinia(), i18n, [VueQueryPlugin, { queryClient: makeQueryClient() }]],
            stubs: {
                VSelect: {
                    name: 'VSelectStub',
                    props: ['modelValue', 'items', 'label'],
                    emits: ['update:modelValue'],
                    template: '<div class="select-stub" />',
                },
            },
        },
    })
}

describe('LangSwitcher', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.stubGlobal('navigator', { language: 'en-US' })
        mockStorage()
        mocked.PATCH.mockReset()
    })

    it('mounts and exposes a select with locale items', () => {
        const w = mountSwitcher()
        expect(w.find('.select-stub').exists()).toBe(true)
    })

    it('updates the local store optimistically and skips backend when anonymous', async () => {
        const w = mountSwitcher()
        const locale = useLocaleStore()
        await w.findComponent({ name: 'VSelectStub' }).vm.$emit('update:modelValue', 'de')
        expect(locale.locale).toBe('de')
        expect(mocked.PATCH).not.toHaveBeenCalled()
    })

    it('persists to backend when authenticated', async () => {
        mocked.PATCH.mockResolvedValueOnce({ data: { data: { display_name: 'A', locale: 'de' } }, error: undefined })
        const w = mountSwitcher()
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [],
        } as never)
        await w.findComponent({ name: 'VSelectStub' }).vm.$emit('update:modelValue', 'de')
        // Wait a microtask for mutation to dispatch.
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.PATCH).toHaveBeenCalled()
    })

    it('ignores non en/de values', async () => {
        const w = mountSwitcher()
        const locale = useLocaleStore()
        const before = locale.locale
        await w.findComponent({ name: 'VSelectStub' }).vm.$emit('update:modelValue', 'fr')
        expect(locale.locale).toBe(before)
    })
})
