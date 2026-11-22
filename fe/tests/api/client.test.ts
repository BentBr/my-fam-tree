/**
 * Drive the openapi-fetch middlewares (familyIdInjector, requestCloner,
 * errorTranslator, warningsBroadcaster, authRefresh) through the public
 * client surface with a stubbed `fetch`. The client knows nothing about
 * stores: session behaviour comes from the hooks passed to
 * `configureClient` (spies from `fakeSessionHooks` here; the real wiring
 * is covered in tests/plugins/apiSession.test.ts).
 */
import { afterEach, beforeEach, describe, expect, it, type Mock, vi } from 'vitest'

import type * as ApiClientModule from '@/api/client'
import type { FamilyId } from '@/types/brand'

import { fakeSessionHooks } from '../helpers/client-session'

async function importClient(): Promise<typeof ApiClientModule> {
    return import('@/api/client')
}

function jsonResponse(status: number, body: unknown, contentType = 'application/json'): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { 'content-type': contentType },
    })
}

const expired = { type: 'about:blank', title: 'Expired', status: 401, code: 'auth_token_expired' }

async function caught(call: Promise<unknown>): Promise<unknown> {
    try {
        await call
    } catch (e) {
        return e
    }
    return undefined
}

describe('@/api/client middlewares', () => {
    let fetchSpy: Mock<typeof fetch>

    beforeEach(() => {
        // openapi-fetch captures globalThis.fetch at client-creation time.
        // Replace fetch BEFORE importing the client module so the closure
        // picks up our spy, and reset modules so each test gets a fresh
        // module (and fresh, unconfigured hooks).
        fetchSpy = vi.fn<typeof fetch>()
        vi.stubGlobal('fetch', fetchSpy)
        vi.resetModules()
    })

    afterEach(() => {
        vi.unstubAllGlobals()
    })

    it('exposes the openapi-fetch verbs', async () => {
        const { client } = await importClient()
        expect(typeof client.GET).toBe('function')
        expect(typeof client.POST).toBe('function')
        expect(typeof client.PATCH).toBe('function')
        expect(typeof client.DELETE).toBe('function')
    })

    it('familyIdInjector sets X-Family-Id from the familyId hook', async () => {
        const { client, configureClient } = await importClient()
        configureClient(fakeSessionHooks({ familyId: () => 'f-1' as FamilyId }))
        fetchSpy.mockResolvedValueOnce(jsonResponse(200, { ok: true }))
        await client.GET('/api/v1/health' as never)
        const req = fetchSpy.mock.calls[0]?.[0] as Request
        expect(req.headers.get('X-Family-Id')).toBe('f-1')
    })

    it('familyIdInjector omits the header when the hook returns null', async () => {
        const { client, configureClient } = await importClient()
        configureClient(fakeSessionHooks())
        fetchSpy.mockResolvedValueOnce(jsonResponse(200, { ok: true }))
        await client.GET('/api/v1/health' as never)
        const req = fetchSpy.mock.calls[0]?.[0] as Request
        expect(req.headers.get('X-Family-Id')).toBeNull()
    })

    it('errorTranslator converts application/problem+json into ApiClientError throws', async () => {
        const { client } = await importClient()
        const { ApiClientError } = await import('@/api/errors')
        fetchSpy.mockResolvedValueOnce(
            jsonResponse(
                400,
                { type: 'about:blank', title: 'Bad', status: 400, code: 'validation_failed' },
                'application/problem+json',
            ),
        )
        expect(await caught(client.GET('/api/v1/health' as never))).toBeInstanceOf(ApiClientError)
    })

    it('warningsBroadcaster passes meta.warnings with the translated message on POST', async () => {
        const { client, configureClient } = await importClient()
        const hooks = fakeSessionHooks()
        configureClient(hooks)
        const warning = { code: 'warning.sibling_partnership', message: 'english fallback', path: null }
        fetchSpy.mockResolvedValueOnce(jsonResponse(200, { data: { id: 'x' }, meta: { warnings: [warning] } }))
        await client.POST(
            '/api/v1/partnerships' as never,
            { body: { partner_a_id: 'a', partner_b_id: 'b', kind: 'marriage' } } as never,
        )
        expect(hooks.notifyWarning).toHaveBeenCalledTimes(1)
        expect(hooks.notifyWarning).toHaveBeenCalledWith(warning, expect.stringContaining('share at least one parent'))
    })

    it('warningsBroadcaster does not fire on GET responses', async () => {
        const { client, configureClient } = await importClient()
        const hooks = fakeSessionHooks()
        configureClient(hooks)
        fetchSpy.mockResolvedValueOnce(
            jsonResponse(200, {
                data: { status: 'ok' },
                meta: { warnings: [{ code: 'warning.parent_child_age_gap_under_14y', message: 'x', path: null }] },
            }),
        )
        await client.GET('/api/v1/health' as never)
        expect(hooks.notifyWarning).not.toHaveBeenCalled()
    })

    it('refreshes on a 401 then retries the original request', async () => {
        const { client, configureClient } = await importClient()
        const hooks = fakeSessionHooks()
        configureClient(hooks)
        fetchSpy
            .mockResolvedValueOnce(jsonResponse(401, expired))
            .mockResolvedValueOnce(jsonResponse(200, { data: { ok: true } }))
        const { data } = await client.GET('/api/v1/relationships' as never)
        expect(data).toEqual({ data: { ok: true } })
        expect(hooks.refreshSession).toHaveBeenCalledTimes(1)
        expect(hooks.endSession).not.toHaveBeenCalled()
        // original + retry; the refresh itself goes through the hook
        expect(fetchSpy).toHaveBeenCalledTimes(2)
    })

    it('shares one in-flight refresh between concurrent 401s', async () => {
        const { client, configureClient } = await importClient()
        let release = (): void => {}
        const pending = new Promise<void>((resolve) => {
            release = resolve
        })
        const hooks = fakeSessionHooks({ refreshSession: vi.fn(() => pending) })
        configureClient(hooks)
        fetchSpy
            .mockResolvedValueOnce(jsonResponse(401, expired))
            .mockResolvedValueOnce(jsonResponse(401, expired))
            .mockImplementation(() => Promise.resolve(jsonResponse(200, { data: { ok: true } })))
        const both = Promise.all([client.GET('/api/v1/relationships' as never), client.GET('/api/v1/persons' as never)])
        await vi.waitFor(() => {
            expect(fetchSpy).toHaveBeenCalledTimes(2)
        })
        // Let both 401 responses reach the refresh gate before it resolves.
        await new Promise((resolve) => setTimeout(resolve, 20))
        release()
        await both
        expect(hooks.refreshSession).toHaveBeenCalledTimes(1)
        expect(fetchSpy).toHaveBeenCalledTimes(4)
    })

    it('ends the session and rethrows the original 401 when the refresh fails', async () => {
        const { client, configureClient } = await importClient()
        const { ApiClientError } = await import('@/api/errors')
        const hooks = fakeSessionHooks({ refreshSession: vi.fn(() => Promise.reject(new Error('refresh failed'))) })
        configureClient(hooks)
        fetchSpy.mockResolvedValueOnce(jsonResponse(401, expired))
        const thrown = await caught(client.GET('/api/v1/relationships' as never))
        expect(thrown).toBeInstanceOf(ApiClientError)
        expect(thrown).toMatchObject({ code: 'auth_token_expired', status: 401 })
        expect(hooks.endSession).toHaveBeenCalledTimes(1)
    })

    it('ends the session when the retried request is still 401', async () => {
        const { client, configureClient } = await importClient()
        const { ApiClientError } = await import('@/api/errors')
        const hooks = fakeSessionHooks()
        configureClient(hooks)
        fetchSpy.mockResolvedValueOnce(jsonResponse(401, expired)).mockResolvedValueOnce(jsonResponse(401, expired))
        expect(await caught(client.GET('/api/v1/relationships' as never))).toBeInstanceOf(ApiClientError)
        expect(hooks.endSession).toHaveBeenCalledTimes(1)
    })

    it('never refreshes a 401 from /auth/refresh itself', async () => {
        const { client, configureClient } = await importClient()
        const hooks = fakeSessionHooks()
        configureClient(hooks)
        fetchSpy.mockResolvedValueOnce(jsonResponse(401, expired))
        await caught(client.POST('/api/v1/auth/refresh'))
        expect(hooks.refreshSession).not.toHaveBeenCalled()
    })

    describe('before configureClient', () => {
        it('sends no family header', async () => {
            const { client } = await importClient()
            fetchSpy.mockResolvedValueOnce(jsonResponse(200, { ok: true }))
            await client.GET('/api/v1/health' as never)
            const req = fetchSpy.mock.calls[0]?.[0] as Request
            expect(req.headers.get('X-Family-Id')).toBeNull()
        })

        it('rethrows a 401 as ApiClientError without retrying', async () => {
            const { client } = await importClient()
            const { ApiClientError } = await import('@/api/errors')
            fetchSpy.mockResolvedValueOnce(jsonResponse(401, expired))
            expect(await caught(client.GET('/api/v1/relationships' as never))).toBeInstanceOf(ApiClientError)
            expect(fetchSpy).toHaveBeenCalledTimes(1)
        })

        it('ignores warnings', async () => {
            const { client } = await importClient()
            fetchSpy.mockResolvedValueOnce(
                jsonResponse(200, { data: {}, meta: { warnings: [{ code: 'warning.x', message: 'x', path: null }] } }),
            )
            const { data } = await client.POST('/api/v1/partnerships' as never, { body: {} } as never)
            expect(data).toEqual({ data: {}, meta: { warnings: [{ code: 'warning.x', message: 'x', path: null }] } })
        })
    })
})
