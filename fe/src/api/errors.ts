import type { components } from './schema'

export type ApiErrorBody = components['schemas']['ApiErrorBody']
export type ErrorCode = components['schemas']['ErrorCode']

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
