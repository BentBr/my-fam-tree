import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/api/client', () => ({
    client: { GET: vi.fn(), POST: vi.fn(), PATCH: vi.fn(), DELETE: vi.fn() },
}))

import { ref } from 'vue'

import { client } from '@/api/client'
import {
    useClearPersonPhoto,
    useCreatePerson,
    useDeletePerson,
    useGetPerson,
    useListPersons,
    useSetFavourite,
    useSetPersonPhoto,
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

describe('useSetPersonPhoto', () => {
    it('POSTs multipart/form-data with a single `file` field to /persons/{id}/photo', async () => {
        mocked.POST.mockResolvedValueOnce({
            data: { data: { photo_key: 'persons/p1/x.jpg', photo_url: 'http://store/p1.jpg' } },
            error: undefined,
        })
        const { result } = makeHookWrapper(() => useSetPersonPhoto())
        const file = new File([new Uint8Array([0xff, 0xd8, 0xff])], 'pic.jpg', { type: 'image/jpeg' })
        await result.mutateAsync({ id: 'p1', file })
        expect(mocked.POST).toHaveBeenCalledTimes(1)
        const [path, opts] = mocked.POST.mock.calls[0] as [
            string,
            { params: { path: { id: string } }; body: FormData; bodySerializer: (b: unknown) => BodyInit },
        ]
        expect(path).toBe('/api/v1/persons/{id}/photo')
        expect(opts.params.path.id).toBe('p1')
        // The body must be a FormData and carry a single `file` entry that
        // round-trips the File the caller passed in. openapi-fetch's body
        // type is `string`, so the hook casts FormData through unknown.
        expect(opts.body).toBeInstanceOf(FormData)
        const sent = opts.body.get('file')
        expect(sent).toBeInstanceOf(File)
        expect((sent as File).name).toBe('pic.jpg')
        // The bodySerializer must hand the FormData through unchanged so
        // fetch can set the multipart boundary itself; rewrapping into
        // JSON.stringify would break the upload.
        expect(opts.bodySerializer(opts.body)).toBe(opts.body)
    })

    it('rejects when the server returns an error envelope', async () => {
        mocked.POST.mockResolvedValueOnce({ data: undefined, error: { msg: 'image.invalid' } })
        const { result } = makeHookWrapper(() => useSetPersonPhoto())
        const file = new File([new Uint8Array([0x00])], 'bad.png', { type: 'image/png' })
        await expect(result.mutateAsync({ id: 'p1', file })).rejects.toBeDefined()
    })
})

describe('useClearPersonPhoto', () => {
    it('DELETEs /persons/{id}/photo', async () => {
        mocked.DELETE.mockResolvedValueOnce({ data: { data: null }, error: undefined })
        const { result } = makeHookWrapper(() => useClearPersonPhoto())
        await result.mutateAsync('p1')
        expect(mocked.DELETE).toHaveBeenCalledWith('/api/v1/persons/{id}/photo', {
            params: { path: { id: 'p1' } },
        })
    })

    it('rejects on error', async () => {
        mocked.DELETE.mockResolvedValueOnce({ data: undefined, error: { msg: 'no' } })
        const { result } = makeHookWrapper(() => useClearPersonPhoto())
        await expect(result.mutateAsync('p1')).rejects.toBeDefined()
    })
})
