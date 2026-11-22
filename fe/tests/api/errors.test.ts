import { describe, expect, it } from 'vitest'

import { ApiClientError, type ApiErrorBody, ApiResponseError, throwIfError } from '@/api/errors'

function body(overrides: Partial<ApiErrorBody> = {}): ApiErrorBody {
    return {
        type: 'about:blank',
        title: 'Bad Request',
        status: 400,
        code: 'validation_failed',
        detail: 'name must be set',
        instance: 'req-abc',
        ...overrides,
    } as ApiErrorBody
}

describe('ApiClientError', () => {
    it('uses detail as the Error message when present', () => {
        const err = new ApiClientError(body({ detail: 'oops' }))
        expect(err.message).toBe('oops')
        expect(err.name).toBe('ApiClientError')
    })

    it('falls back to title when detail is null', () => {
        const err = new ApiClientError(body({ detail: null }))
        expect(err.message).toBe('Bad Request')
    })

    it('exposes code, status, and body', () => {
        const b = body()
        const err = new ApiClientError(b)
        expect(err.code).toBe(b.code)
        expect(err.status).toBe(b.status)
        expect(err.body).toBe(b)
    })

    it('is an instance of Error', () => {
        expect(new ApiClientError(body())).toBeInstanceOf(Error)
    })
})

describe('throwIfError', () => {
    it('does nothing when there is no error', () => {
        expect(() => {
            throwIfError(undefined)
        }).not.toThrow()
    })

    it('rethrows an Error unchanged', () => {
        const err = new ApiClientError(body())
        expect(() => {
            throwIfError(err)
        }).toThrow(err)
    })

    it('wraps a non-Error body in an ApiResponseError that keeps the body', () => {
        let caught: unknown
        try {
            throwIfError({ msg: 'boom' })
        } catch (e) {
            caught = e
        }
        expect(caught).toBeInstanceOf(ApiResponseError)
        expect(caught).toMatchObject({ name: 'ApiResponseError', body: { msg: 'boom' } })
    })

    it('wraps a null body too (only undefined means "no error")', () => {
        expect(() => {
            throwIfError(null)
        }).toThrow(ApiResponseError)
    })
})
