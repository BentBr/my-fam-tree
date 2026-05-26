import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn(), DELETE: vi.fn() },
}))

import { ref } from 'vue'

import { client } from '@/api/client'
import {
    useCreatePerson,
    useDeletePerson,
    useGetPerson,
    useListPersons,
    useSetFavourite,
    useUpdatePerson,
    type PersonInput,
} from '@/api/hooks/persons'

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

describe('useListPersons', () => {
    it('GETs and unwraps .data', async () => {
        mocked.GET.mockResolvedValueOnce({ data: { data: [{ id: 'p1' }] }, error: undefined })
        const { result } = makeHookWrapper(() => useListPersons())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/persons')
        expect(result.data.value).toEqual([{ id: 'p1' }])
    })

    it('errors when data is empty (204-like)', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result } = makeHookWrapper(() => useListPersons())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value?.message).toMatch(/empty response/)
    })

    it('errors when error is set', async () => {
        mocked.GET.mockResolvedValueOnce({ data: undefined, error: { msg: 'fail' } })
        const { result } = makeHookWrapper(() => useListPersons())
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(result.error.value).toBeDefined()
    })
})

describe('useGetPerson', () => {
    it('GETs /persons/{id} with the ref id when enabled', async () => {
        mocked.GET.mockResolvedValueOnce({
            data: { data: { id: 'p1', given_name: 'A' } },
            error: undefined,
        })
        const id = ref<string | null>('p1')
        const { result } = makeHookWrapper(() => useGetPerson(id))
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).toHaveBeenCalledWith('/api/v1/persons/{id}', {
            params: { path: { id: 'p1' } },
        })
        expect(result.data.value).toEqual({ id: 'p1', given_name: 'A' })
    })

    it('skips the fetch when id is null', async () => {
        const id = ref<string | null>(null)
        makeHookWrapper(() => useGetPerson(id))
        await new Promise<void>((r) => setTimeout(r, 5))
        expect(mocked.GET).not.toHaveBeenCalled()
    })
})

describe('useCreatePerson', () => {
    it('POSTs body and returns data', async () => {
        mocked.POST.mockResolvedValueOnce({ data: { data: { id: 'new' } }, error: undefined })
        const { result } = makeHookWrapper(() => useCreatePerson())
        const input: PersonInput = { given_name: 'A' }
        const out = await result.mutateAsync(input)
        expect(out).toEqual({ id: 'new' })
    })

    it('rejects on empty data', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result } = makeHookWrapper(() => useCreatePerson())
        await expect(result.mutateAsync({ given_name: 'A' })).rejects.toThrow(/empty response/)
    })

    it('rejects on error', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useCreatePerson())
        await expect(result.mutateAsync({ given_name: 'A' })).rejects.toBeDefined()
    })
})

describe('useUpdatePerson', () => {
    it('PATCHes with path id + body', async () => {
        mocked.PATCH.mockResolvedValueOnce({ data: { data: { id: 'p1' } }, error: undefined })
        const { result } = makeHookWrapper(() => useUpdatePerson())
        const out = await result.mutateAsync({ id: 'p1', input: { given_name: 'New' } })
        expect(mocked.PATCH).toHaveBeenCalledWith('/api/v1/persons/{id}', {
            params: { path: { id: 'p1' } },
            body: { given_name: 'New' },
        })
        expect(out.id).toBe('p1')
    })

    it('rejects on empty data', async () => {
        mocked.PATCH.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result } = makeHookWrapper(() => useUpdatePerson())
        await expect(result.mutateAsync({ id: 'p1', input: {} })).rejects.toThrow(/empty response/)
    })

    it('rejects on error', async () => {
        mocked.PATCH.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useUpdatePerson())
        await expect(result.mutateAsync({ id: 'p', input: {} })).rejects.toBeDefined()
    })
})

describe('useDeletePerson', () => {
    it('DELETEs and resolves on success', async () => {
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: undefined })
        const { result } = makeHookWrapper(() => useDeletePerson())
        await expect(result.mutateAsync('p1')).resolves.toBeUndefined()
        expect(mocked.DELETE).toHaveBeenCalledWith('/api/v1/persons/{id}', {
            params: { path: { id: 'p1' } },
        })
    })

    it('rejects on error', async () => {
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useDeletePerson())
        await expect(result.mutateAsync('p1')).rejects.toBeDefined()
    })
})

describe('useSetFavourite', () => {
    it('PATCHes the favourite endpoint with the new state', async () => {
        mocked.PATCH.mockResolvedValueOnce({ data: { data: { is_favourite_for_me: true } }, error: undefined })
        const { result } = makeHookWrapper(() => useSetFavourite())
        await result.mutateAsync({ id: 'p1', isFavourite: true })
        expect(mocked.PATCH).toHaveBeenCalledWith('/api/v1/persons/{id}/favourite', {
            params: { path: { id: 'p1' } },
            body: { is_favourite: true },
        })
    })

    it('optimistically flips is_favourite_for_me on the cached tree node', async () => {
        // Never-resolving PATCH so we can observe the optimistic state
        // between onMutate and settle.
        mocked.PATCH.mockReturnValueOnce(new Promise(() => {}))
        const { result, queryClient } = makeHookWrapper(() => useSetFavourite())
        queryClient.setQueryData(['tree'], {
            nodes: [
                { id: 'p1', is_favourite_for_me: false },
                { id: 'p2', is_favourite_for_me: false },
            ],
            edges: [],
        })
        result.mutate({ id: 'p1', isFavourite: true })
        // Let onMutate run.
        await new Promise((r) => setTimeout(r, 0))
        const tree = queryClient.getQueryData<{ nodes: { id: string; is_favourite_for_me: boolean }[] }>(['tree'])
        expect(tree?.nodes.find((n) => n.id === 'p1')?.is_favourite_for_me).toBe(true)
        // Untouched node stays as-is.
        expect(tree?.nodes.find((n) => n.id === 'p2')?.is_favourite_for_me).toBe(false)
    })

    it('rolls the tree cache back when the PATCH rejects', async () => {
        mocked.PATCH.mockResolvedValueOnce({ data: undefined, error: { msg: 'boom' } })
        const { result, queryClient } = makeHookWrapper(() => useSetFavourite())
        queryClient.setQueryData(['tree'], {
            nodes: [{ id: 'p1', is_favourite_for_me: false }],
            edges: [],
        })
        await expect(result.mutateAsync({ id: 'p1', isFavourite: true })).rejects.toBeDefined()
        const tree = queryClient.getQueryData<{ nodes: { id: string; is_favourite_for_me: boolean }[] }>(['tree'])
        // Rolled back to the pre-mutation value.
        expect(tree?.nodes.find((n) => n.id === 'p1')?.is_favourite_for_me).toBe(false)
    })
})
