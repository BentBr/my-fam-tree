import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter, type RouteRecordRaw } from 'vue-router'

// `import type * as` (rather than inline `typeof import(...)` types) so
// `@typescript-eslint/consistent-type-imports` accepts the type-only side
// of the mock setup.
import type * as InvitesHooks from '@/api/hooks/invites'

type InviteRow = InvitesHooks.InviteRow

const cancelMutate = vi.fn()
const invitesData = ref<InviteRow[] | undefined>(undefined)
const invitesIsLoading = ref(false)
const invitesError = ref<unknown>(null)

vi.mock('@/api/hooks/invites', async (importOriginal) => {
    const actual = await importOriginal<typeof InvitesHooks>()
    return {
        ...actual,
        useInvites: () => ({ data: invitesData, isLoading: invitesIsLoading, error: invitesError }),
        useCancelInvite: () => ({ mutate: cancelMutate, isPending: ref(false) }),
    }
})
vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn(), DELETE: vi.fn() } }))

import { i18n } from '@/i18n'
import AdminInvites from '@/views/admin/AdminInvites.vue'

const stubs = {
    'v-skeleton-loader': { template: '<div class="skel" :data-type="$attrs.type" />' },
    'v-alert': { template: '<div class="alert" :data-type="$attrs.type"><slot /></div>' },
    'v-card': { template: '<div class="card"><slot /></div>' },
    'v-card-title': { template: '<div><slot /></div>' },
    'v-card-text': { template: '<div><slot /></div>' },
    'v-card-actions': { template: '<div><slot /></div>' },
    'v-spacer': { template: '<div />' },
    'v-table': {
        template: '<table :data-testid="$attrs[\'data-testid\']"><slot /></table>',
    },
    'v-dialog': {
        template:
            '<div v-if="modelValue" class="dialog" :data-testid="$attrs[\'data-testid\']"><slot /></div>',
        props: ['modelValue'],
    },
    'v-btn': {
        // Forward `$attrs` so the `data-testid` and `@click` parent listener
        // both reach the real <button>. Vue passes click listeners through
        // $attrs as well, so `<button>` re-emitting is unnecessary.
        template: '<button v-bind="$attrs"><slot /></button>',
    },
}

async function mountView() {
    const routes: RouteRecordRaw[] = [
        { path: '/admin/invites', component: { template: '<div />' } },
        { path: '/tree', component: { template: '<div />' } },
    ]
    const router = createRouter({ history: createMemoryHistory(), routes })
    await router.push('/admin/invites')
    await router.isReady()
    return mount(AdminInvites, { global: { plugins: [createPinia(), i18n, router], stubs } })
}

function row(over: Partial<InviteRow> = {}): InviteRow {
    return {
        id: 'inv-1',
        email: 'guest@example.com',
        role: 'user',
        person_id: null,
        expires_at: '2030-01-15T00:00:00Z',
        invited_by: 'u-admin',
        ...over,
    }
}

describe('AdminInvites', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        cancelMutate.mockReset()
        invitesData.value = undefined
        invitesIsLoading.value = false
        invitesError.value = null
    })

    it('renders the loading skeleton while the query is in flight', async () => {
        invitesIsLoading.value = true
        const w = await mountView()
        expect(w.find('.skel').exists()).toBe(true)
    })

    it('renders an error alert when the query fails', async () => {
        invitesError.value = new Error('boom')
        const w = await mountView()
        expect(w.find('.alert').exists()).toBe(true)
    })

    it('renders the empty-state row when there are no invites', async () => {
        invitesData.value = []
        const w = await mountView()
        expect(w.find('[data-testid="admin-invites-empty"]').exists()).toBe(true)
        expect(w.findAll('tbody tr').length).toBe(1)
    })

    it('renders an invite row per row in the data + cancel buttons per row', async () => {
        invitesData.value = [
            row({ id: 'a' }),
            row({ id: 'b', email: 'b@x.com', role: 'admin' }),
        ]
        const w = await mountView()
        expect(w.find('[data-testid="admin-invites-row-a"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-invites-row-b"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-invites-cancel-a"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-invites-cancel-b"]').exists()).toBe(true)
    })

    it('renders a person-link for invites that carry a person_id; em-dash otherwise', async () => {
        invitesData.value = [
            row({ id: 'with', person_id: 'person-uuid' }),
            row({ id: 'without', person_id: null }),
        ]
        const w = await mountView()
        expect(w.find('[data-testid="admin-invites-person-with"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-invites-person-without"]').exists()).toBe(false)
    })

    it('clicking cancel opens the confirm dialog, confirming calls the mutation, cancelling does not', async () => {
        invitesData.value = [row({ id: 'inv-cancel' })]
        const w = await mountView()
        // Dialog hidden before the user clicks cancel.
        expect(w.find('[data-testid="admin-invites-confirm-dialog"]').exists()).toBe(false)

        await w.find('[data-testid="admin-invites-cancel-inv-cancel"]').trigger('click')
        await flushPromises()
        expect(w.find('[data-testid="admin-invites-confirm-dialog"]').exists()).toBe(true)
        expect(cancelMutate).not.toHaveBeenCalled()

        await w.find('[data-testid="admin-invites-confirm"]').trigger('click')
        await flushPromises()
        expect(cancelMutate).toHaveBeenCalledExactlyOnceWith('inv-cancel')
        // Dialog closes after confirming — `confirmId` is reset to null.
        expect(w.find('[data-testid="admin-invites-confirm-dialog"]').exists()).toBe(false)
    })
})
