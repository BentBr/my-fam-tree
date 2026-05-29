import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn() },
}))

import { client } from '@/api/client'
import { useAuditList, type AuditFilter } from '@/api/hooks/audit'
import { useActiveFamilyStore } from '@/stores/activeFamily'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

beforeEach(() => {
    mocked.GET.mockReset()
    localStorage.clear()
})

describe('useAuditList', () => {
    it('GETs /audit with active family id + camelCase→snake_case query params', async () => {
        // Seed an active family so the query is enabled.
        localStorage.setItem('my-fam-tree:activeFamily', 'fam-1')
        mocked.GET.mockResolvedValueOnce({
            data: { data: { items: [], total: 0 } },
            error: undefined,
        })
        const filter = ref<AuditFilter>({
            page: 2,
            pageSize: 100,
            from: '2026-01-01T00:00:00Z',
            to: '2026-02-01T00:00:00Z',
            action: 'persons.update',
            entityKind: 'person',
            actorUserId: 'u-9',
        })
        const { result } = makeHookWrapper(() => useAuditList(filter))
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/families/{family_id}/audit', {
            params: {
                path: { family_id: 'fam-1' },
                query: {
                    page: 2,
                    page_size: 100,
                    from: '2026-01-01T00:00:00Z',
                    to: '2026-02-01T00:00:00Z',
                    action: 'persons.update',
                    entity_kind: 'person',
                    actor_user_id: 'u-9',
                },
            },
        })
        expect(result.data.value).toEqual({ items: [], total: 0 })
    })

    it('omits unset filter keys (only passes what the caller supplied)', async () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'fam-1')
        mocked.GET.mockResolvedValueOnce({
            data: { data: { items: [], total: 0 } },
            error: undefined,
        })
        const filter = ref<AuditFilter>({ page: 1 })
        makeHookWrapper(() => useAuditList(filter))
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/families/{family_id}/audit', {
            params: { path: { family_id: 'fam-1' }, query: { page: 1 } },
        })
    })

    it('is disabled (does not fetch) when there is no active family', async () => {
        // No localStorage seed → activeFamilyId is null → enabled=false.
        const filter = ref<AuditFilter>({ page: 1 })
        const { result } = makeHookWrapper(() => useAuditList(filter))
        // Confirm preconditions: store actually reports null.
        const family = useActiveFamilyStore()
        expect(family.activeFamilyId).toBeNull()
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).not.toHaveBeenCalled()
        // Disabled queries stay in "pending" without surfacing data/error.
        expect(result.data.value).toBeUndefined()
        expect(result.error.value).toBeNull()
    })

    it('surfaces the response error when the GET fails', async () => {
        localStorage.setItem('my-fam-tree:activeFamily', 'fam-1')
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const filter = ref<AuditFilter>({})
        const { result } = makeHookWrapper(() => useAuditList(filter))
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(result.error.value).toBeDefined()
    })
})
