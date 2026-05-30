import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn() } }))

import { client } from '@/api/client'
import LanguageMenu from '@/components/common/LanguageMenu.vue'
import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'

interface MockedClient {
    PATCH: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

function stubStorage(): void {
    const store: Record<string, string> = {}
    vi.stubGlobal('localStorage', {
        getItem: (k: string) => store[k] ?? null,
        setItem: (k: string, v: string) => {
            store[k] = v
        },
        removeItem: (k: string) => {
            delete store[k]
        },
        clear: () => {
            for (const k of Object.keys(store)) delete store[k]
        },
        key: (i: number) => Object.keys(store)[i] ?? null,
        get length() {
            return Object.keys(store).length
        },
    })
}

const stubs = {
    'v-menu': {
        template: '<div class="m"><slot name="activator" :props="{}" /><slot /></div>',
    },
    'v-btn': {
        template: '<button class="trigger" :aria-label="$attrs[\'aria-label\']"><slot /></button>',
    },
    'v-list': { template: '<ul class="l"><slot /></ul>' },
    'v-list-item': {
        template:
            '<li :data-testid="$attrs[\'data-testid\']" :data-active="$attrs.active" @click="$emit(\'click\')"><slot name="prepend" /><span class="t">{{ title }}</span></li>',
        props: ['title'],
        emits: ['click'],
    },
}

describe('LanguageMenu', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        stubStorage()
        mocked.PATCH = vi.fn().mockResolvedValue({ data: { data: {} } })
    })

    function makeQueryClient(): QueryClient {
        return new QueryClient({ defaultOptions: { queries: { retry: 0 }, mutations: { retry: 0 } } })
    }

    it('renders the active locale flag on the trigger', () => {
        const w = mount(LanguageMenu, {
            global: {
                plugins: [i18n, [VueQueryPlugin, { queryClient: makeQueryClient() }]],
                stubs,
            },
        })
        const trigger = w.find('button.trigger')
        expect(trigger.attributes('aria-label')).toBeDefined()
        expect(w.text()).toContain('🇬🇧')
    })

    it('flips the locale store on click', async () => {
        const locale = useLocaleStore()
        const w = mount(LanguageMenu, {
            global: {
                plugins: [i18n, [VueQueryPlugin, { queryClient: makeQueryClient() }]],
                stubs,
            },
        })
        await w.find('[data-testid="language-menu-de"]').trigger('click')
        expect(locale.locale).toBe('de')
    })

    it('does not PATCH /users/me for anonymous callers', async () => {
        const auth = useAuthStore()
        expect(auth.status).toBe('anonymous')
        const w = mount(LanguageMenu, {
            global: {
                plugins: [i18n, [VueQueryPlugin, { queryClient: makeQueryClient() }]],
                stubs,
            },
        })
        await w.find('[data-testid="language-menu-de"]').trigger('click')
        expect(mocked.PATCH).not.toHaveBeenCalled()
    })

    it('persists the locale to the backend when authenticated', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({ user_id: 'u', email: 'a@b', locale: 'en', families: [] } as never)
        const w = mount(LanguageMenu, {
            global: {
                plugins: [i18n, [VueQueryPlugin, { queryClient: makeQueryClient() }]],
                stubs,
            },
        })
        await w.find('[data-testid="language-menu-de"]').trigger('click')
        expect(mocked.PATCH).toHaveBeenCalledTimes(1)
    })
})
