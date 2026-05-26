import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'

vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn() } }))

import { i18n } from '@/i18n'
import AdminLayout from '@/layouts/AdminLayout.vue'

// AdminLayout's rail:
// 1. Does NOT mount the main NavDrawer. Admin pages are a focused,
//    role-gated surface with their own side-rail; surfacing the global
//    nav alongside it produced two competing nav columns. The
//    `admin-rail-back` link is the canonical escape hatch back to /tree.
// 2. Render each rail entry as a Vuetify `v-list-item` (not a raw <a>)
//    so the visual treatment matches the main nav drawer.
// 3. Carry a "Back to tree" affordance at the top.
describe('AdminLayout', () => {
    async function mountLayout(): Promise<ReturnType<typeof mount>> {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [
                { path: '/', component: { template: '<div />' } },
                { path: '/admin/audit', component: { template: '<div data-testid="audit-stub" />' } },
                { path: '/admin/members', component: { template: '<div />' } },
                { path: '/admin/invites', component: { template: '<div />' } },
                { path: '/tree', component: { template: '<div />' } },
            ],
        })
        await router.push('/admin/audit')
        await router.isReady()
        return mount(AdminLayout, {
            global: {
                plugins: [createPinia(), i18n, router],
                stubs: {
                    AppBar: { template: '<div class="appbar-stub" />' },
                    NavDrawer: { template: '<div class="nav-stub" data-testid="nav-drawer-stub" />' },
                    'v-main': { template: '<main><slot /></main>' },
                    'v-container': { template: '<div><slot /></div>' },
                    'v-divider': { template: '<hr />' },
                    'v-list': { template: '<div class="v-list"><slot /></div>' },
                    // Mirror v-list-item enough that the test can read the
                    // resolved `to`, `prepend-icon`, and slot content.
                    'v-list-item': {
                        template:
                            '<a class="v-list-item" :data-testid="$attrs[\'data-testid\']" :data-to="to" :data-icon="prependIcon" :data-disabled="disabled">{{ title }}<slot /></a>',
                        props: ['to', 'prependIcon', 'title', 'color', 'disabled'],
                    },
                },
            },
        })
    }

    it('does NOT mount the main NavDrawer on admin pages', async () => {
        const w = await mountLayout()
        expect(w.find('[data-testid="nav-drawer-stub"]').exists()).toBe(false)
        expect(w.find('[data-testid="admin-rail"]').exists()).toBe(true)
    })

    it('renders a Back-to-tree rail item routing to /tree', async () => {
        const w = await mountLayout()
        const back = w.find('[data-testid="admin-rail-back"]')
        expect(back.exists()).toBe(true)
        expect(back.attributes('data-to')).toBe('/tree')
        expect(back.attributes('data-icon')).toBe('arrow-left')
    })

    it('renders each rail item as a v-list-item with the right route + icon', async () => {
        const w = await mountLayout()
        const members = w.find('[data-testid="admin-rail-members"]')
        const invites = w.find('[data-testid="admin-rail-invites"]')
        const audit = w.find('[data-testid="admin-rail-audit"]')
        expect(members.exists()).toBe(true)
        expect(members.attributes('data-to')).toBe('/admin/members')
        expect(members.attributes('data-icon')).toBe('users')
        expect(invites.exists()).toBe(true)
        expect(invites.attributes('data-to')).toBe('/admin/invites')
        expect(invites.attributes('data-icon')).toBe('mail')
        expect(audit.exists()).toBe(true)
        expect(audit.attributes('data-to')).toBe('/admin/audit')
        expect(audit.attributes('data-icon')).toBe('list')
    })
})
