import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))
// AppBar now reads /users/me for the avatar — stub the hook so the test
// doesn't drag tanstack-query + the auth gate into the mount.
vi.mock('@/api/hooks/users', () => ({
    useMe: () => ({ data: ref(undefined), isLoading: ref(false), error: ref(null) }),
}))

import AppBar from '@/components/layout/AppBar.vue'
import { i18n } from '@/i18n'
import { useAuthStore } from '@/stores/auth'

async function mountAppBar() {
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
    return mount(AppBar, {
        global: {
            plugins: [i18n, router],
            stubs: {
                'v-app-bar': { template: '<div><slot /></div>' },
                'v-app-bar-nav-icon': {
                    template: '<button class="navicon" @click="$emit(\'click\')" />',
                    emits: ['click'],
                },
                'v-app-bar-title': { template: '<div class="title"><slot /></div>' },
                'v-spacer': { template: '<div />' },
                'v-menu': { template: '<div class="menu"><slot name="activator" :props="{}" /><slot /></div>' },
                'v-btn': {
                    template: '<button class="btn" @click="$emit(\'click\')"><slot /></button>',
                    emits: ['click'],
                },
                'v-list': { template: '<div><slot /></div>' },
                'v-list-item': {
                    template: '<div class="li" :data-testid="$attrs[\'data-testid\']" @click="$emit(\'click\')" />',
                    emits: ['click'],
                },
                FamilySwitcher: { template: '<div />' },
                LangSwitcher: { template: '<div />' },
                DefaultAvatar: { template: '<div data-testid="default-avatar-stub" />' },
            },
        },
    })
}

describe('AppBar', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    it('renders title', async () => {
        const w = await mountAppBar()
        expect(w.find('.title').exists()).toBe(true)
    })

    it('shows the user menu only when authenticated', async () => {
        const w1 = await mountAppBar()
        // Anonymous: v-menu is conditional
        expect(w1.find('.menu').exists()).toBe(false)

        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [],
        } as never)
        const w2 = await mountAppBar()
        expect(w2.find('.menu').exists()).toBe(true)
    })

    it('signOut calls logout + routes to /auth/sign-in', async () => {
        const auth = useAuthStore()
        auth.applyClaimsPayload({
            user_id: 'u',
            email: 'a@b',
            locale: 'en',
            families: [],
        } as never)
        const w = await mountAppBar()
        const signOutItem = w.find('[data-testid="sign-out"]')
        expect(signOutItem.exists()).toBe(true)
        await signOutItem.trigger('click')
        await flushPromises()
        expect(auth.status).toBe('anonymous')
    })
})
