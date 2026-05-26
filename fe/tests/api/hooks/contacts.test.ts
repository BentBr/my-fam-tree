import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn(), DELETE: vi.fn() },
}))

import { client } from '@/api/client'
import {
    useContacts,
    useCreateContact,
    useDeleteContact,
    useUpdateContact,
    type ContactInput,
} from '@/api/hooks/contacts'

import { makeHookWrapper } from '../../helpers/hook-wrapper'

interface MockedClient {
    GET: ReturnType<typeof vi.fn>
    POST: ReturnType<typeof vi.fn>
    PATCH: ReturnType<typeof vi.fn>
    DELETE: ReturnType<typeof vi.fn>
}
const mocked = client as unknown as MockedClient

beforeEach(() => {
    mocked.GET.mockReset()
    mocked.POST.mockReset()
    mocked.PATCH.mockReset()
    mocked.DELETE.mockReset()
})

describe('useContacts', () => {
    it('GETs /persons/{id}/contacts and unwraps data.data.contacts', async () => {
        mocked.GET.mockResolvedValueOnce({
            data: { data: { contacts: [{ id: 'c-1', kind: 'email', value: 'a@b' }] } },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useContacts('p-1'))
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/persons/{id}/contacts', {
            params: { path: { id: 'p-1' } },
        })
        expect(result.data.value).toEqual([{ id: 'c-1', kind: 'email', value: 'a@b' }])
    })

    it('is disabled (does not fetch) when personId is empty', async () => {
        const pid = ref('')
        makeHookWrapper(() => useContacts(pid))
        await new Promise<void>((r) => setTimeout(r, 10))
        expect(mocked.GET).not.toHaveBeenCalled()
    })
})

describe('useCreateContact', () => {
    it('POSTs to /persons/{id}/contacts with body and invalidates the contacts cache', async () => {
        mocked.POST.mockResolvedValueOnce({
            data: { data: { id: 'c-new', kind: 'email', value: 'x@y' } },
            error: undefined,
        })
        const { result, queryClient } = makeHookWrapper(() => useCreateContact('p-1'))
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        const input: ContactInput = { kind: 'email', value: 'x@y' }
        const out = await result.mutateAsync(input)
        expect(mocked.POST).toHaveBeenCalledWith('/api/v1/persons/{id}/contacts', {
            params: { path: { id: 'p-1' } },
            body: input,
        })
        expect(out).toEqual({ id: 'c-new', kind: 'email', value: 'x@y' })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['contacts', 'p-1'] })
    })

    it('rejects on error', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const { result } = makeHookWrapper(() => useCreateContact('p-1'))
        await expect(
            result.mutateAsync({ kind: 'email', value: 'a@b' } as ContactInput),
        ).rejects.toBeDefined()
    })
})

describe('useUpdateContact', () => {
    it('PATCHes /contacts/{id} with the supplied body and invalidates cache', async () => {
        mocked.PATCH.mockResolvedValueOnce({
            data: { data: { id: 'c-1', kind: 'email', value: 'new@b' } },
            error: undefined,
        })
        const { result, queryClient } = makeHookWrapper(() => useUpdateContact('p-1'))
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        const input: ContactInput = { kind: 'email', value: 'new@b' }
        await result.mutateAsync({ id: 'c-1', input })
        expect(mocked.PATCH).toHaveBeenCalledWith('/api/v1/contacts/{id}', {
            params: { path: { id: 'c-1' } },
            body: input,
        })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['contacts', 'p-1'] })
    })

    it('rejects on error', async () => {
        mocked.PATCH.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const { result } = makeHookWrapper(() => useUpdateContact('p-1'))
        await expect(
            result.mutateAsync({ id: 'c-1', input: { kind: 'email', value: 'a@b' } as ContactInput }),
        ).rejects.toBeDefined()
    })
})

describe('useDeleteContact', () => {
    it('DELETEs /contacts/{id} and invalidates the contacts cache', async () => {
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result, queryClient } = makeHookWrapper(() => useDeleteContact('p-1'))
        const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
        await result.mutateAsync('c-1')
        expect(mocked.DELETE).toHaveBeenCalledWith('/api/v1/contacts/{id}', {
            params: { path: { id: 'c-1' } },
        })
        expect(invalidate).toHaveBeenCalledWith({ queryKey: ['contacts', 'p-1'] })
    })

    it('rejects on error', async () => {
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const { result } = makeHookWrapper(() => useDeleteContact('p-1'))
        await expect(result.mutateAsync('c-1')).rejects.toBeDefined()
    })
})
