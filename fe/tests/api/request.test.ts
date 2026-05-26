import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/i18n', () => ({
    i18n: { global: { t: (k: string) => `t:${k}` } },
}))

import { expectOk, unwrap, useApiMutation } from '@/api/request'
import { useUiStore } from '@/stores/ui'

import { makeHookWrapper } from '../helpers/hook-wrapper'

describe('unwrap', () => {
    it('returns the inner data on success', async () => {
        await expect(unwrap(Promise.resolve({ data: { data: { id: 'x' } }, error: undefined }))).resolves.toEqual({
            id: 'x',
        })
    })

    it('returns null when the inner data is null (nullable endpoints)', async () => {
        await expect(unwrap(Promise.resolve({ data: { data: null }, error: undefined }))).resolves.toBeNull()
    })

    it('throws the error when one is present', async () => {
        await expect(unwrap(Promise.resolve({ data: undefined, error: { msg: 'boom' } }))).rejects.toEqual({
            msg: 'boom',
        })
    })

    it('throws on an empty envelope (undefined data, no error)', async () => {
        await expect(unwrap(Promise.resolve({ data: undefined, error: undefined }))).rejects.toThrow(/empty response/)
    })
})

describe('expectOk', () => {
    it('resolves to void on success', async () => {
        await expect(expectOk(Promise.resolve({ error: undefined }))).resolves.toBeUndefined()
    })

    it('throws the error when present', async () => {
        await expect(expectOk(Promise.resolve({ error: { msg: 'no' } }))).rejects.toEqual({ msg: 'no' })
    })
})

describe('useApiMutation', () => {
    beforeEach(() => {
        vi.restoreAllMocks()
    })

    it('pushes a success toast from a string key + invalidates the listed keys', async () => {
        const { result, queryClient } = makeHookWrapper(() =>
            useApiMutation<{ id: string }, { ok: true }>({
                mutationFn: () => Promise.resolve({ ok: true }),
                success: 'toasts.thing_done',
                invalidate: () => [['things'], ['other']],
            }),
        )
        const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')
        const ui = useUiStore()
        await result.mutateAsync({ id: 'a' })
        expect(ui.toasts.at(-1)?.message).toBe('t:toasts.thing_done')
        expect(ui.toasts.at(-1)?.kind).toBe('success')
        expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['things'] })
        expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['other'] })
    })

    it('resolves the success key via a function of the vars', async () => {
        const { result } = makeHookWrapper(() =>
            useApiMutation<{ on: boolean }, void>({
                mutationFn: () => Promise.resolve(),
                success: (vars) => (vars.on ? 'toasts.on' : 'toasts.off'),
            }),
        )
        const ui = useUiStore()
        await result.mutateAsync({ on: false })
        expect(ui.toasts.at(-1)?.message).toBe('t:toasts.off')
    })

    it('runs the extra onSuccess callback with vars + data', async () => {
        const seen: Array<[unknown, unknown]> = []
        const { result } = makeHookWrapper(() =>
            useApiMutation<{ id: string }, { v: number }>({
                mutationFn: () => Promise.resolve({ v: 7 }),
                onSuccess: (vars, data) => seen.push([vars, data]),
            }),
        )
        await result.mutateAsync({ id: 'z' })
        expect(seen).toEqual([[{ id: 'z' }, { v: 7 }]])
    })

    it('does not toast or invalidate when the mutation rejects', async () => {
        const { result, queryClient } = makeHookWrapper(() =>
            useApiMutation<void, void>({
                mutationFn: () => Promise.reject(new Error('fail')),
                success: 'toasts.nope',
                invalidate: () => [['things']],
            }),
        )
        const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')
        const ui = useUiStore()
        await expect(result.mutateAsync()).rejects.toThrow('fail')
        expect(ui.toasts).toHaveLength(0)
        expect(invalidateSpy).not.toHaveBeenCalled()
    })
})
