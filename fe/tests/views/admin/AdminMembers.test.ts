import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

// `import type * as` (rather than inline `typeof import(...)` types) so
// `@typescript-eslint/consistent-type-imports` accepts the type-only side
// of the mock setup.
import type * as MembersHooks from '@/api/hooks/members'
import type * as OwnerTransferHooks from '@/api/hooks/owner_transfer'

type MemberRow = MembersHooks.MemberRow
type TransferStatusRow = OwnerTransferHooks.TransferStatusRow

const setRoleMutate = vi.fn()
const revokeMutate = vi.fn()
const beginTransferMutate = vi.fn().mockResolvedValue(undefined)
const cancelTransferMutate = vi.fn()

const membersData = ref<MemberRow[] | undefined>(undefined)
const membersIsLoading = ref(false)
const membersError = ref<unknown>(null)
const transferData = ref<TransferStatusRow | null | undefined>(null)

vi.mock('@/api/hooks/members', async (importOriginal) => {
    const actual = await importOriginal<typeof MembersHooks>()
    return {
        ...actual,
        useMembers: () => ({ data: membersData, isLoading: membersIsLoading, error: membersError }),
        useSetRole: () => ({ mutate: setRoleMutate, isPending: ref(false) }),
        useRevokeMember: () => ({ mutate: revokeMutate, isPending: ref(false) }),
    }
})
vi.mock('@/api/hooks/owner_transfer', async (importOriginal) => {
    const actual = await importOriginal<typeof OwnerTransferHooks>()
    return {
        ...actual,
        useOwnerTransfer: () => ({ data: transferData, isLoading: ref(false), error: ref(null) }),
        useBeginOwnerTransfer: () => ({ mutateAsync: beginTransferMutate, isPending: ref(false) }),
        useCancelOwnerTransfer: () => ({ mutate: cancelTransferMutate, isPending: ref(false) }),
    }
})
vi.mock('@/api/client', () => ({ client: { GET: vi.fn(), POST: vi.fn(), DELETE: vi.fn() } }))

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'
import { i18n } from '@/i18n'
import AdminMembers from '@/views/admin/AdminMembers.vue'
import type { FamilyId } from '@/types/brand'

const stubs = {
    'v-skeleton-loader': { template: '<div class="skel" />' },
    'v-alert': { template: '<div class="alert" :data-type="$attrs.type"><slot /></div>' },
    'v-card': { template: '<div class="card"><slot /></div>' },
    'v-card-title': { template: '<div><slot /></div>' },
    'v-card-text': { template: '<div><slot /></div>' },
    'v-card-actions': { template: '<div><slot /></div>' },
    'v-spacer': { template: '<div />' },
    'v-chip': { template: '<span class="chip" :data-color="color"><slot /></span>', props: ['color'] },
    'v-table': { template: '<table :data-testid="$attrs[\'data-testid\']"><slot /></table>' },
    'v-dialog': {
        template:
            '<div v-if="modelValue" class="dialog" :data-testid="$attrs[\'data-testid\']"><slot /></div>',
        props: ['modelValue'],
    },
    'v-btn': { template: '<button v-bind="$attrs"><slot /></button>' },
}

function row(over: Partial<MemberRow> = {}): MemberRow {
    return {
        user_id: 'u-other',
        email: 'other@x.com',
        display_name: 'Other Person',
        linked_person_name: null,
        role: 'user',
        joined_at: '2025-01-01T00:00:00Z',
        ...over,
    }
}

interface SetupArgs {
    myRole?: 'owner' | 'admin' | 'user' | null
    myUserId?: string | null
    transfer?: TransferStatusRow | null
    members?: MemberRow[] | undefined
    loading?: boolean
    error?: unknown
}

async function mountView(args: SetupArgs = {}) {
    setActivePinia(createPinia())
    const auth = useAuthStore()
    const family = useActiveFamilyStore()
    if (args.myUserId !== null && args.myUserId !== undefined) {
        auth.applyClaimsPayload({
            user_id: args.myUserId,
            email: 'me@x.com',
            locale: 'en',
            families: [{ id: 'fam-1', name: 'Müller', role: args.myRole ?? 'owner' }],
        })
        family.setActive('fam-1' as FamilyId)
    }
    membersData.value = args.members
    membersIsLoading.value = args.loading ?? false
    membersError.value = args.error ?? null
    transferData.value = args.transfer ?? null
    const w = mount(AdminMembers, { global: { plugins: [i18n], stubs } })
    await flushPromises()
    return w
}

describe('AdminMembers', () => {
    beforeEach(() => {
        setRoleMutate.mockReset()
        revokeMutate.mockReset()
        beginTransferMutate.mockReset().mockResolvedValue(undefined)
        cancelTransferMutate.mockReset()
    })

    it('renders the loading skeleton while the query is in flight', async () => {
        const w = await mountView({ loading: true, myUserId: 'u-me', myRole: 'owner' })
        expect(w.find('.skel').exists()).toBe(true)
    })

    it('renders an error alert when the query fails', async () => {
        const w = await mountView({
            error: new Error('boom'),
            myUserId: 'u-me',
            myRole: 'owner',
        })
        // The error alert is the only `<v-alert>` rendered in this branch
        // (the pending-transfer banner is also a v-alert but it's gated on
        // `pendingTransfer !== null`, which is null here).
        const alerts = w.findAll('.alert')
        expect(alerts.length).toBe(1)
        expect(alerts[0]?.attributes('data-type')).toBe('error')
    })

    it('renders the empty-state row when there are no members', async () => {
        const w = await mountView({ members: [], myUserId: 'u-me', myRole: 'owner' })
        expect(w.find('[data-testid="admin-members-empty"]').exists()).toBe(true)
    })

    it('owner sees promote/demote/revoke/transfer affordances on the right rows', async () => {
        // Owner viewer, three other rows: a regular user, an admin, the
        // owner row itself. Each surfaces a different action set.
        const members: MemberRow[] = [
            row({ user_id: 'u-user', role: 'user', display_name: 'Reg User' }),
            row({ user_id: 'u-admin', role: 'admin', display_name: 'Adm Admin' }),
            row({ user_id: 'u-me', role: 'owner', display_name: 'Olwen Owner' }),
        ]
        const w = await mountView({ members, myUserId: 'u-me', myRole: 'owner' })
        // Promote on the user row only — admins are already at the cap
        // for the role matrix this UI exposes.
        expect(w.find('[data-testid="admin-members-promote-u-user"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-members-promote-u-admin"]').exists()).toBe(false)
        // Demote only on the admin row, and only for owners.
        expect(w.find('[data-testid="admin-members-demote-u-admin"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-members-demote-u-user"]').exists()).toBe(false)
        // Revoke on user + admin rows, NOT on the owner's own row.
        expect(w.find('[data-testid="admin-members-revoke-u-user"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-members-revoke-u-admin"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-members-revoke-u-me"]').exists()).toBe(false)
        // Transfer offered only to admins (and only when there's no
        // pending transfer); not to the owner-themselves row.
        expect(w.find('[data-testid="admin-members-transfer-u-admin"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-members-transfer-u-user"]').exists()).toBe(false)
        expect(w.find('[data-testid="admin-members-transfer-u-me"]').exists()).toBe(false)
    })

    it('admin viewer cannot demote or transfer (owner-only actions)', async () => {
        const members: MemberRow[] = [
            row({ user_id: 'u-user', role: 'user' }),
            row({ user_id: 'u-admin', role: 'admin' }),
            row({ user_id: 'u-owner', role: 'owner' }),
        ]
        const w = await mountView({ members, myUserId: 'u-me-admin', myRole: 'admin' })
        // Admins can promote and revoke users (their cap) but not demote
        // other admins or transfer ownership.
        expect(w.find('[data-testid="admin-members-promote-u-user"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-members-demote-u-admin"]').exists()).toBe(false)
        expect(w.find('[data-testid="admin-members-transfer-u-admin"]').exists()).toBe(false)
        // Admins can revoke users only (not admins / not the owner).
        expect(w.find('[data-testid="admin-members-revoke-u-user"]').exists()).toBe(true)
        expect(w.find('[data-testid="admin-members-revoke-u-admin"]').exists()).toBe(false)
        expect(w.find('[data-testid="admin-members-revoke-u-owner"]').exists()).toBe(false)
    })

    it('promote calls setRole with admin; no confirm dialog (one-click)', async () => {
        const members = [row({ user_id: 'u-promote', role: 'user' })]
        const w = await mountView({ members, myUserId: 'u-me', myRole: 'owner' })
        await w.find('[data-testid="admin-members-promote-u-promote"]').trigger('click')
        expect(setRoleMutate).toHaveBeenCalledExactlyOnceWith({
            userId: 'u-promote',
            role: 'admin',
        })
    })

    it('demote opens the confirm dialog, confirms with setRole user', async () => {
        const members = [row({ user_id: 'u-dem', role: 'admin' })]
        const w = await mountView({ members, myUserId: 'u-me', myRole: 'owner' })
        await w.find('[data-testid="admin-members-demote-u-dem"]').trigger('click')
        await flushPromises()
        expect(w.find('[data-testid="admin-members-confirm-dialog"]').exists()).toBe(true)
        expect(setRoleMutate).not.toHaveBeenCalled()

        await w.find('[data-testid="admin-members-confirm"]').trigger('click')
        await flushPromises()
        expect(setRoleMutate).toHaveBeenCalledExactlyOnceWith({
            userId: 'u-dem',
            role: 'user',
        })
        // Dialog closes after confirm.
        expect(w.find('[data-testid="admin-members-confirm-dialog"]').exists()).toBe(false)
    })

    it('revoke opens the confirm dialog, confirms with revoke', async () => {
        const members = [row({ user_id: 'u-rev', role: 'user' })]
        const w = await mountView({ members, myUserId: 'u-me', myRole: 'owner' })
        await w.find('[data-testid="admin-members-revoke-u-rev"]').trigger('click')
        await flushPromises()
        await w.find('[data-testid="admin-members-confirm"]').trigger('click')
        await flushPromises()
        expect(revokeMutate).toHaveBeenCalledExactlyOnceWith('u-rev')
    })

    it('transfer dialog submits to beginTransfer.mutateAsync with the target user_id', async () => {
        const members = [row({ user_id: 'u-target', role: 'admin' })]
        const w = await mountView({ members, myUserId: 'u-me', myRole: 'owner' })
        await w.find('[data-testid="admin-members-transfer-u-target"]').trigger('click')
        await flushPromises()
        expect(w.find('[data-testid="admin-members-transfer-dialog"]').exists()).toBe(true)
        await w.find('[data-testid="admin-members-transfer-submit"]').trigger('click')
        await flushPromises()
        expect(beginTransferMutate).toHaveBeenCalledExactlyOnceWith('u-target')
    })

    it('pending-transfer banner renders when one is in flight; cancel calls cancelTransfer', async () => {
        const members = [row({ user_id: 'u-target', role: 'admin' })]
        const transfer: TransferStatusRow = {
            id: 't-1',
            from_user_id: 'u-me',
            to_user_id: 'u-target',
            from_confirmed: true,
            to_confirmed: false,
            expires_at: '2030-01-01T00:00:00Z',
        }
        const w = await mountView({ members, transfer, myUserId: 'u-me', myRole: 'owner' })
        expect(w.find('[data-testid="admin-members-transfer-banner"]').exists()).toBe(true)
        // Transfer-button hidden when a transfer is already pending.
        expect(w.find('[data-testid="admin-members-transfer-u-target"]').exists()).toBe(false)
        await w.find('[data-testid="admin-members-transfer-cancel"]').trigger('click')
        expect(cancelTransferMutate).toHaveBeenCalledExactlyOnceWith()
    })
})
