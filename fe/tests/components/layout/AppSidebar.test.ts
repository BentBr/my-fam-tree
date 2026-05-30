import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter, type RouteRecordRaw } from 'vue-router'

vi.mock('vuetify', () => ({
    useDisplay: () => ({ smAndDown: ref(false) }),
}))

import AppSidebar from '@/components/layout/AppSidebar.vue'
import { i18n } from '@/i18n'

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

// `to` and `title` are bound from the parent as attrs. Don't declare
// them as Vue props on the stub — that hides them from `$attrs`, and
// the test reads them via `$attrs.to`.
const stubs = {
    'v-navigation-drawer': {
        template: '<aside class="drawer" :data-testid="$attrs[\'data-testid\']"><slot /><slot name="append" /></aside>',
    },
    'v-list': { template: '<ul><slot /></ul>' },
    'v-list-item': {
        template: '<li :data-testid="$attrs[\'data-testid\']" :data-to="$attrs.to">{{ $attrs.title }}</li>',
    },
    'v-icon': { template: '<i :data-icon="$attrs.icon" />' },
    RouterLink: { template: '<a><slot /></a>' },
}

async function mountSidebar(meta: Record<string, unknown>) {
    const routes: RouteRecordRaw[] = [{ path: '/', component: { template: '<div />' }, meta }]
    const router = createRouter({ history: createMemoryHistory(), routes })
    await router.push('/')
    await router.isReady()
    return mount(AppSidebar, { global: { plugins: [createPinia(), i18n, router], stubs } })
}

describe('AppSidebar', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        stubStorage()
    })

    it('renders nothing when route.meta.sidebar is unset (public surfaces)', async () => {
        const w = await mountSidebar({})
        expect(w.find('[data-testid="nav-drawer"]').exists()).toBe(false)
    })

    it('main variant shows Tree + Upcoming, no Admin without that role', async () => {
        const w = await mountSidebar({ sidebar: 'main' })
        const items = w.findAll('li')
        const tos = items.map((i) => i.attributes('data-to'))
        expect(tos).toContain('/tree')
        expect(tos).toContain('/upcoming')
        expect(w.find('[data-testid="nav-admin"]').exists()).toBe(false)
    })

    it('admin variant shows back-to-tree + admin rail items', async () => {
        const w = await mountSidebar({ sidebar: 'admin' })
        const tos = w.findAll('li').map((i) => i.attributes('data-to'))
        expect(tos).toContain('/tree')
        expect(tos).toContain('/admin/members')
        expect(tos).toContain('/admin/invites')
        expect(tos).toContain('/admin/audit')
    })
})
