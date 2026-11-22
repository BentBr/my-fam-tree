import type { components } from './schema'

export type ApiErrorBody = components['schemas']['ApiErrorBody']
export type ErrorCode = components['schemas']['ErrorCode']
export type FieldViolation = components['schemas']['FieldViolation']
export type Warning = components['schemas']['Warning']

export class ApiClientError extends Error {
    readonly code: ErrorCode
    readonly status: number
    readonly body: ApiErrorBody

    constructor(body: ApiErrorBody) {
        super(body.detail ?? body.title)
        this.name = 'ApiClientError'
        this.code = body.code
        this.status = body.status
        this.body = body
    }
}

/**
 * A non-problem error response surfaced by openapi-fetch as `{ error }`
 * (problem+json responses already reject with `ApiClientError` in the
 * `errorTranslator` middleware). Keeps the raw body for debugging.
 */
export class ApiResponseError extends Error {
    readonly body: unknown

    constructor(body: unknown) {
        super('unexpected API error response')
        this.name = 'ApiResponseError'
        this.body = body
    }
}

/** Throws openapi-fetch's `error` field, if set, as an `Error`. */
export function throwIfError(error: unknown): void {
    if (error === undefined) return
    throw error instanceof Error ? error : new ApiResponseError(error)
}
