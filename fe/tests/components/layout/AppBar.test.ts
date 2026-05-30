import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))
// Atoms are stubbed below; we don't need /users/me here either.
vi.mock('@/api/hooks/users', () => ({
    useMe: () => ({ data: ref(undefined), isLoading: ref(false), error: ref(null) }),
}))
// AppBar now reads `useDisplay().smAndDown` to fold inline tools into
// the AccountControl menu on phones. Mocking the helper avoids dragging
// the full Vuetify plugin into the harness — we always test the
// desktop layout (smAndDown=false) since the inline-tools logic is
// boolean-simple.
vi.mock('vuetify', () => ({
    useDisplay: () => ({ smAndDown: ref(false) }),
}))

import AppBar from '@/components/layout/AppBar.vue'
import { i18n } from '@/i18n'

interface MountOpts {
    sidebar?: 'main' | 'admin' | 'none'
}

async function mountAppBar(opts: MountOpts = {}) {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            {
                path: '/',
                component: { template: '<div />' },
                // Only set `meta.sidebar` when the test asked for one; the
                // type rejects `undefined` for an exact-optional field, so
                // either supply the value or leave the key off entirely.
                meta: opts.sidebar === undefined ? {} : { sidebar: opts.sidebar },
            },
            { path: '/auth/sign-in', component: { template: '<div />' } },
        ],
    })
    await router.push('/')
    await router.isReady()
    return mount(AppBar, {
        global: {
            plugins: [i18n, router],
            stubs: {
                'v-app-bar': { template: '<div class="appbar"><slot /></div>' },
                'v-app-bar-nav-icon': {
                    template: '<button class="navicon" data-testid="nav-toggle" @click="$emit(\'click\')" />',
                    emits: ['click'],
                },
                'v-spacer': { template: '<div class="spacer" />' },
                BrandLogo: { template: '<div class="brand-stub" data-testid="brand-logo" />' },
                FamilySwitcher: { template: '<div class="family-stub" />' },
                ThemeToggle: { template: '<div class="theme-stub" data-testid="theme-toggle" />' },
                LanguageMenu: { template: '<div class="lang-stub" data-testid="language-menu" />' },
                AccountControl: { template: '<div class="account-stub" data-testid="user-menu" />' },
            },
        },
    })
}

describe('AppBar', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    it('renders the brand logo, theme toggle, language menu, and account control on every route', async () => {
        const w = await mountAppBar({ sidebar: 'none' })
        expect(w.findAll('[data-testid="brand-logo"]')).toHaveLength(1)
        expect(w.findAll('[data-testid="theme-toggle"]')).toHaveLength(1)
        expect(w.findAll('[data-testid="language-menu"]')).toHaveLength(1)
        expect(w.findAll('[data-testid="user-menu"]')).toHaveLength(1)
    })

    it('hides the hamburger on routes without a sidebar', async () => {
        const w = await mountAppBar({ sidebar: 'none' })
        expect(w.find('[data-testid="nav-toggle"]').exists()).toBe(false)
    })

    it('shows the hamburger on routes that mount the main sidebar', async () => {
        const w = await mountAppBar({ sidebar: 'main' })
        expect(w.find('[data-testid="nav-toggle"]').exists()).toBe(true)
    })

    it('shows the hamburger on admin-variant routes too', async () => {
        const w = await mountAppBar({ sidebar: 'admin' })
        expect(w.find('[data-testid="nav-toggle"]').exists()).toBe(true)
    })
})
