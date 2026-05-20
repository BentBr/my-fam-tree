import createClient, { type Middleware } from 'openapi-fetch'

import { useActiveFamilyStore } from '@/stores/activeFamily'
import { useAuthStore } from '@/stores/auth'

import { ApiClientError, type ApiErrorBody } from './errors'
import type { paths } from './schema'

const baseUrl = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080'

const familyIdInjector: Middleware = {
    async onRequest({ request }) {
        const family = useActiveFamilyStore()
        const id = family.activeFamilyId
        if (id !== null && !request.headers.has('X-Family-Id')) {
            request.headers.set('X-Family-Id', id)
        }
        return request
    },
}

let refreshing: Promise<void> | null = null

const authRefresh: Middleware = {
    async onResponse({ request, response }) {
        if (response.status !== 401) return response
        let body: ApiErrorBody | undefined
        try {
            body = (await response.clone().json()) as ApiErrorBody
        } catch {
            return response
        }
        if (body.code !== 'auth_token_expired') {
            throw new ApiClientError(body)
        }
        const auth = useAuthStore()
        refreshing ??= auth
            .refresh()
            .catch((e) => {
                throw e
            })
            .finally(() => {
                refreshing = null
            })
        await refreshing
        return fetch(request)
    },
}

const errorTranslator: Middleware = {
    async onResponse({ response }) {
        if (response.ok) return response
        const ct = response.headers.get('content-type') ?? ''
        if (!ct.includes('application/problem+json')) return response
        const body = (await response.clone().json()) as ApiErrorBody
        throw new ApiClientError(body)
    },
}

export const client = createClient<paths>({ baseUrl, credentials: 'include' })
client.use(familyIdInjector, authRefresh, errorTranslator)
