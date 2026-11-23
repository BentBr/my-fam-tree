import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn() } }))

import { client } from '@/api/client'
import LanguageMenu from '@/components/common/LanguageMenu.vue'
import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'
import { memoryStorage } from '../../helpers/memory-storage'

interface MockedClient {
    PATCH: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

function stubStorage(): void {
    vi.stubGlobal('localStorage', memoryStorage())
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

    // The component swaps the live route's `/en`|`/de` prefix on pick, so
    // the harness mounts it inside a minimal locale-prefixed router.
    async function mountMenu(): Promise<{ w: ReturnType<typeof mount>; router: Router }> {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/:lang(en|de)', name: 'home', component: { template: '<div />' } }],
        })
        await router.push('/en')
        await router.isReady()
        const w = mount(LanguageMenu, {
            global: {
                plugins: [i18n, router, [VueQueryPlugin, { queryClient: makeQueryClient() }]],
                stubs,
            },
        })
        return { w, router }
    }

    it('renders the active locale flag on the trigger', async () => {
        const { w } = await mountMenu()
        const trigger = w.find('button.trigger')
        expect(trigger.attributes('aria-label')).toBeDefined()
        expect(w.text()).toContain('🇬🇧')
    })

    it('flips the locale store and swaps the URL prefix on click', async () => {
        const locale = useLocaleStore()
        const { w, router } = await mountMenu()
        await w.find('[data-testid="language-menu-de"]').trigger('click')
        expect(locale.locale).toBe('de')
        // URL follows the pick — the route prefix is the locale's source
        // of truth, so the in-place replace must carry the new lang. The
        // component fires `router.replace` without awaiting it; flush the
        // microtask queue so the navigation settles before asserting.
        await flushPromises()
        expect(router.currentRoute.value.path).toBe('/de')
    })

    it('does not PATCH /users/me for anonymous callers', async () => {
        const auth = useAuthStore()
        expect(auth.status).toBe('anonymous')
        const { w } = await mountMenu()
        await w.find('[data-testid="language-menu-de"]').trigger('click')
        expect(mocked.PATCH).not.toHaveBeenCalled()
    })

    it('persists the locale to the backend when authenticated', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({ user_id: 'u', email: 'a@b', locale: 'en', edge_style: 'curved', families: [] })
        const { w } = await mountMenu()
        await w.find('[data-testid="language-menu-de"]').trigger('click')
        expect(mocked.PATCH).toHaveBeenCalledTimes(1)
    })
})
