import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))
// `useMe` is the source of avatar + display-name; stub so the test
// drives the activator branch via the auth store alone.
const meRef = ref<{ avatar_url: string | null; display_name: string | null } | undefined>(undefined)
vi.mock('@/api/hooks/users', () => ({
    useMe: () => ({ data: meRef, isLoading: ref(false), error: ref(null) }),
}))
// `useDisplay` decides whether the mobile fold-in items render. Default
// to desktop (smAndDown=false) so the dropdown only carries the
// auth-state-driven items.
vi.mock('vuetify', () => ({
    useDisplay: () => ({ smAndDown: ref(false) }),
}))

import AccountControl from '@/components/common/AccountControl.vue'
import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'

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
        template:
            '<button class="trigger" :data-testid="$attrs[\'data-testid\']" :aria-label="$attrs[\'aria-label\']"><slot /></button>',
    },
    'v-icon': { template: '<i class="icon" :data-icon="$attrs.icon" />' },
    'v-list': { template: '<ul class="l"><slot /></ul>' },
    'v-list-item': {
        template:
            '<li :data-testid="$attrs[\'data-testid\']" :data-to="$attrs.to" @click="$emit(\'click\')"><slot /></li>',
        props: ['title', 'to'],
        emits: ['click'],
    },
    'v-list-subheader': { template: '<li class="h"><slot /></li>' },
    'v-divider': { template: '<hr />' },
    DefaultAvatar: { template: '<div class="avatar-stub" />' },
}

async function mountControl() {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: { template: '<div />' } },
            { path: '/auth/sign-in', component: { template: '<div />' } },
            { path: '/account', component: { template: '<div />' } },
        ],
    })
    await router.push('/')
    await router.isReady()
    const qc = new QueryClient({ defaultOptions: { queries: { retry: 0 }, mutations: { retry: 0 } } })
    return mount(AccountControl, {
        global: {
            plugins: [i18n, router, [VueQueryPlugin, { queryClient: qc }]],
            stubs,
        },
    })
}

describe('AccountControl', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        stubStorage()
        meRef.value = undefined
    })

    it('anonymous: shows the orange user icon and the Login + Register items', async () => {
        const w = await mountControl()
        expect(w.find('[data-icon="user"]').exists()).toBe(true)
        expect(w.find('.avatar-stub').exists()).toBe(false)
        expect(w.find('[data-testid="account-login"]').exists()).toBe(true)
        expect(w.find('[data-testid="account-register"]').exists()).toBe(true)
    })

    it('authenticated + no display name: still shows the orange user icon', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({ user_id: 'u', email: 'a@b', locale: 'en', families: [] } as never)
        meRef.value = { avatar_url: null, display_name: null }
        const w = await mountControl()
        expect(w.find('[data-icon="user"]').exists()).toBe(true)
        expect(w.find('.avatar-stub').exists()).toBe(false)
    })

    it('authenticated + display name set: shows the DefaultAvatar', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({ user_id: 'u', email: 'a@b', locale: 'en', families: [] } as never)
        meRef.value = { avatar_url: null, display_name: 'Anna Müller' }
        const w = await mountControl()
        expect(w.find('.avatar-stub').exists()).toBe(true)
        expect(w.find('[data-icon="user"]').exists()).toBe(false)
    })

    it('authenticated: dropdown carries Account + Sign-out, not Login/Register', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({ user_id: 'u', email: 'a@b', locale: 'en', families: [] } as never)
        const w = await mountControl()
        expect(w.find('[data-testid="user-menu-account"]').exists()).toBe(true)
        expect(w.find('[data-testid="sign-out"]').exists()).toBe(true)
        expect(w.find('[data-testid="account-login"]').exists()).toBe(false)
    })
})
