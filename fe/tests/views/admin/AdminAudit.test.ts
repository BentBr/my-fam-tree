import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter, type RouteRecordRaw } from 'vue-router'

interface AuditRow {
    id: string
    created_at: string
    actor_email?: string | null
    actor_display_name?: string | null
    action: string
    entity_kind: string
    entity_id?: string | null
    entity_person_id?: string | null
    entity_person_name?: string | null
    metadata?: unknown
}

const auditData = ref<{ data: AuditRow[]; total: number } | undefined>(undefined)
const auditIsLoading = ref(false)
const auditError = ref<unknown>(null)

// Capture the most recent filter the component passes in so we can verify
// the page-reset-on-filter-change behaviour.
let lastFilter: unknown = null

// `import type * as` (rather than inline `typeof import(...)` types) so
// `@typescript-eslint/consistent-type-imports` accepts the type-only side
// of the mock setup.
import type * as AuditHooks from '@/api/hooks/audit'

vi.mock('@/api/hooks/audit', async (importOriginal) => {
    const actual = await importOriginal<typeof AuditHooks>()
    return {
        ...actual,
        useAuditList: (filter: unknown) => {
            // Mirror the real hook's `toValue` so a ref/computed is unwrapped
            // — the component passes a `computed<AuditFilter>`.
            const f = filter as { value?: unknown }
            lastFilter = f?.value ?? f
            return { data: auditData, isLoading: auditIsLoading, error: auditError }
        },
    }
})
vi.mock('@/api/client', () => ({ client: { GET: vi.fn() } }))

import { i18n } from '@/i18n'
import AdminAudit from '@/views/admin/AdminAudit.vue'

const stubs = {
    'v-skeleton-loader': { template: '<div class="skel" />' },
    'v-alert': { template: '<div class="alert" :data-type="$attrs.type"><slot /></div>' },
    'v-card': { template: '<div class="card"><slot /></div>' },
    'v-table': { template: '<table :data-testid="$attrs[\'data-testid\']"><slot /></table>' },
    'v-pagination': {
        template:
            '<nav :data-testid="$attrs[\'data-testid\']" :data-length="length" :data-page="modelValue" />',
        props: ['modelValue', 'length'],
    },
    // Lightweight stand-ins for v-text-field / v-select that mirror v-model
    // by re-emitting `update:modelValue` on a plain input change.
    'v-text-field': {
        template:
            '<input :data-testid="$attrs[\'data-testid\']" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
        props: ['modelValue'],
        emits: ['update:modelValue'],
    },
    // The AdminAudit tests never drive v-select interaction (the
    // filter-change + date-widening paths are covered by the existing
    // admin-audit.test.ts e2e, see the comment on that test). Rendering
    // a placeholder `<select>` with the testid is enough to keep the
    // page tree healthy.
    'v-select': {
        template: '<select :data-testid="$attrs[\'data-testid\']" />',
        props: ['modelValue', 'items'],
    },
    'v-btn': { template: '<button v-bind="$attrs"><slot /></button>' },
}

async function mountView() {
    setActivePinia(createPinia())
    const routes: RouteRecordRaw[] = [
        { path: '/admin/audit', component: { template: '<div />' } },
        { path: '/tree', component: { template: '<div />' } },
    ]
    const router = createRouter({ history: createMemoryHistory(), routes })
    await router.push('/admin/audit')
    await router.isReady()
    const w = mount(AdminAudit, { global: { plugins: [i18n, router], stubs } })
    await flushPromises()
    return { wrapper: w, router }
}

function row(over: Partial<AuditRow> = {}): AuditRow {
    return {
        id: 'a-1',
        created_at: '2025-06-01T10:30:00Z',
        actor_display_name: 'Alice',
        actor_email: 'alice@example.com',
        action: 'create',
        entity_kind: 'person',
        entity_id: 'p-1',
        entity_person_id: 'p-1',
        entity_person_name: 'Klaus Müller',
        metadata: {},
        ...over,
    }
}

describe('AdminAudit', () => {
    beforeEach(() => {
        auditData.value = undefined
        auditIsLoading.value = false
        auditError.value = null
        lastFilter = null
    })

    it('renders the loading skeleton while the query is in flight', async () => {
        auditIsLoading.value = true
        const { wrapper } = await mountView()
        expect(wrapper.find('.skel').exists()).toBe(true)
    })

    it('renders an error alert when the query fails', async () => {
        auditError.value = new Error('boom')
        const { wrapper } = await mountView()
        expect(wrapper.find('.alert').exists()).toBe(true)
    })

    it('renders the empty-state row when there are no audit entries', async () => {
        auditData.value = { data: [], total: 0 }
        const { wrapper } = await mountView()
        expect(wrapper.find('[data-testid="admin-audit-empty"]').exists()).toBe(true)
    })

    it('renders one row per audit entry + the entity-link for rows with entity_person_id', async () => {
        auditData.value = {
            data: [
                row({ id: 'r-linked' }),
                row({
                    id: 'r-unlinked',
                    entity_person_id: null,
                    entity_person_name: null,
                }),
            ],
            total: 2,
        }
        const { wrapper } = await mountView()
        expect(wrapper.find('[data-testid="admin-audit-row-r-linked"]').exists()).toBe(true)
        expect(wrapper.find('[data-testid="admin-audit-row-r-unlinked"]').exists()).toBe(true)
        expect(wrapper.find('[data-testid="admin-audit-entity-r-linked"]').exists()).toBe(true)
        // The unlinked row falls back to the entity-kind label and does NOT
        // render the clickable anchor.
        expect(wrapper.find('[data-testid="admin-audit-entity-r-unlinked"]').exists()).toBe(false)
    })

    it('clicking the entity-link routes to /tree?center=<person_id>', async () => {
        auditData.value = { data: [row({ id: 'r-link', entity_person_id: 'person-uuid' })], total: 1 }
        const { wrapper, router } = await mountView()
        const push = vi.spyOn(router, 'push')
        await wrapper.find('[data-testid="admin-audit-entity-r-link"]').trigger('click')
        await flushPromises()
        expect(push).toHaveBeenCalledExactlyOnceWith({ path: '/tree', query: { center: 'person-uuid' } })
    })

    it('invite rows surface email + role from metadata as a secondary line', async () => {
        auditData.value = {
            data: [
                row({
                    id: 'inv',
                    action: 'invite',
                    entity_kind: 'membership',
                    metadata: { email: 'new@x.com', role: 'admin' },
                }),
            ],
            total: 1,
        }
        const { wrapper } = await mountView()
        expect(wrapper.find('[data-testid="admin-audit-invite-details-inv"]').exists()).toBe(true)
        // An invite row whose metadata is missing both fields renders no
        // secondary line — the `inviteMetadata` coerce returns null.
        auditData.value = {
            data: [row({ id: 'inv-empty', action: 'invite', entity_kind: 'membership', metadata: {} })],
            total: 1,
        }
        const w2 = await mountView()
        expect(w2.wrapper.find('[data-testid="admin-audit-invite-details-inv-empty"]').exists()).toBe(false)
    })

    it('initial filter is page=1 + default pageSize, clear filters keeps page=1', async () => {
        auditData.value = { data: [row()], total: 200 }
        const { wrapper } = await mountView()
        // After mount, the filter sees page=1 + the default size.
        expect(lastFilter).toMatchObject({ page: 1, pageSize: 50 })

        // Clear filters with no active fields is a no-op for `lastFilter`'s
        // top-level keys but exercises the clear handler so the row stays
        // covered. The action/entityKind/from/to keys are absent because
        // the AuditFilter only sets present fields.
        await wrapper.find('[data-testid="admin-audit-filter-clear"]').trigger('click')
        await flushPromises()
        expect(lastFilter).toMatchObject({ page: 1, pageSize: 50 })
        expect(lastFilter).not.toHaveProperty('action')
        expect(lastFilter).not.toHaveProperty('from')
    })

    // The filter-change → page-reset behaviour and the date-widening
    // (`fromDate`+T00:00:00Z / `toDate`+T23:59:59Z) is exercised end-to-end
    // by `fe/e2e/tests/admin-audit.test.ts`. Reproducing it in a unit
    // shell would require driving Vuetify's real v-select / v-text-field
    // event chain (a stub's `setValue` doesn't propagate through to the
    // parent's v-model), which buys very little over the e2e.

    it('pagination pageCount = ceil(total / pageSize) with a 1-row floor', async () => {
        // 173 rows at 50/page → 4 pages.
        auditData.value = { data: [row()], total: 173 }
        const { wrapper } = await mountView()
        expect(wrapper.find('[data-testid="admin-audit-paginator"]').attributes('data-length')).toBe('4')

        // 0 rows still renders a 1-page paginator (avoids an empty paginator).
        auditData.value = { data: [], total: 0 }
        const w2 = await mountView()
        expect(w2.wrapper.find('[data-testid="admin-audit-paginator"]').attributes('data-length')).toBe('1')
    })
})
