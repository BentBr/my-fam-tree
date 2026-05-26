/**
 * Drive the three openapi-fetch middlewares (familyIdInjector, authRefresh,
 * errorTranslator) by stubbing `fetch` and issuing client calls. The
 * middlewares are anonymous closures registered via `client.use(...)` and
 * can only be exercised through the public client surface.
 *
 * openapi-fetch runs middlewares in registration order on the response. The
 * tests below cover:
 *   - Happy 200 → all 3 middlewares pass through cleanly + verb surface is wired.
 *   - 401 with application/problem+json: authRefresh decides whether to refresh
 *     or end-session+throw based on the body's `code`.
 *   - 4xx with application/problem+json: errorTranslator converts to ApiClientError.
 */
import { setActivePinia, createPinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const routerReplace = vi.fn()
vi.mock('@/router', () => ({ router: { replace: routerReplace } }))

function mockStorage(): void {
    const store: Record<string, string> = {}
    vi.stubGlobal('localStorage', {
        getItem: (k: string) => store[k] ?? null,
        setItem: (k: string, v: string) => {
            store[k] = v
        },
        removeItem: (k: string) => {
            delete store[k]
        },
    })
}

function jsonResponse(status: number, body: unknown, contentType = 'application/json'): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { 'content-type': contentType },
    })
}

describe('@/api/client middlewares', () => {
    let fetchSpy: ReturnType<typeof vi.fn>

    beforeEach(() => {
        vi.stubGlobal('navigator', { language: 'en-US' })
        mockStorage()
        setActivePinia(createPinia())
        // openapi-fetch captures globalThis.fetch at client-creation time.
        // Replace fetch BEFORE importing the client module so the closure
        // picks up our spy, and reset modules so each test gets a fresh
        // module + closure.
        fetchSpy = vi.fn()
        vi.stubGlobal('fetch', fetchSpy)
        vi.resetModules()
        routerReplace.mockReset()
    })

    afterEach(() => {
        vi.unstubAllGlobals()
    })

    it('exposes the openapi-fetch verbs', async () => {
        fetchSpy.mockResolvedValueOnce(jsonResponse(200, { ok: true }))
        const { client } = await import('@/api/client')
        expect(typeof client.GET).toBe('function')
        expect(typeof client.POST).toBe('function')
        expect(typeof client.PATCH).toBe('function')
        expect(typeof client.DELETE).toBe('function')
    })

    it('familyIdInjector adds X-Family-Id when active family is set', async () => {
        localStorage.setItem('my-family:activeFamily', 'f-1')
        const { client } = await import('@/api/client')
        fetchSpy.mockResolvedValueOnce(jsonResponse(200, { ok: true }))
        await client.GET('/api/v1/health' as never)
        const req = fetchSpy.mock.calls[0]?.[0] as Request
        expect(req.headers.get('X-Family-Id')).toBe('f-1')
    })

    it('familyIdInjector omits header when no active family', async () => {
        const { client } = await import('@/api/client')
        fetchSpy.mockResolvedValueOnce(jsonResponse(200, { ok: true }))
        await client.GET('/api/v1/health' as never)
        const req = fetchSpy.mock.calls[0]?.[0] as Request
        expect(req.headers.get('X-Family-Id')).toBeNull()
    })

    it('errorTranslator converts application/problem+json into ApiClientError throws', async () => {
        const { client } = await import('@/api/client')
        const { ApiClientError } = await import('@/api/errors')
        fetchSpy.mockResolvedValueOnce(
            jsonResponse(
                400,
                {
                    type: 'about:blank',
                    title: 'Bad',
                    status: 400,
                    code: 'validation_failed',
                },
                'application/problem+json',
            ),
        )
        let thrown: unknown
        try {
            await client.GET('/api/v1/health' as never)
        } catch (e) {
            thrown = e
        }
        expect(thrown).toBeInstanceOf(ApiClientError)
    })

    it('warningsBroadcaster surfaces meta.warnings as info toasts on POST', async () => {
        const { client } = await import('@/api/client')
        const { useUiStore } = await import('@/stores/ui')
        fetchSpy.mockResolvedValueOnce(
            jsonResponse(200, {
                data: { id: 'x' },
                meta: {
                    warnings: [
                        {
                            code: 'warning.sibling_partnership',
                            message: 'english fallback',
                            path: null,
                        },
                    ],
                },
            }),
        )
        await client.POST(
            '/api/v1/partnerships' as never,
            {
                body: { partner_a_id: 'a', partner_b_id: 'b', kind: 'marriage' },
            } as never,
        )
        const toasts = useUiStore().toasts
        expect(toasts).toHaveLength(1)
        expect(toasts[0]?.kind).toBe('info')
        expect(toasts[0]?.code).toBe('warning.sibling_partnership')
        expect(toasts[0]?.message).toContain('share at least one parent')
    })

    it('warningsBroadcaster does not fire on GET responses', async () => {
        const { client } = await import('@/api/client')
        const { useUiStore } = await import('@/stores/ui')
        fetchSpy.mockResolvedValueOnce(
            jsonResponse(200, {
                data: { status: 'ok' },
                meta: {
                    warnings: [{ code: 'warning.parent_child_age_gap_under_14y', message: 'x', path: null }],
                },
            }),
        )
        await client.GET('/api/v1/health' as never)
        expect(useUiStore().toasts).toHaveLength(0)
    })

    it('authRefresh throws ApiClientError on a non-expired 401', async () => {
        const { client } = await import('@/api/client')
        const { ApiClientError } = await import('@/api/errors')
        fetchSpy.mockResolvedValueOnce(
            jsonResponse(
                401,
                {
                    type: 'about:blank',
                    title: 'Unauthorized',
                    status: 401,
                    code: 'auth_required',
                },
                'application/problem+json',
            ),
        )
        let thrown: unknown
        try {
            await client.GET('/api/v1/health' as never)
        } catch (e) {
            thrown = e
        }
        // The middleware throws ApiClientError regardless of whether
        // endSession mutates the (anonymous) store. The auth-side
        // mutation is covered directly in tests/stores/auth.test.ts.
        expect(thrown).toBeInstanceOf(ApiClientError)
    })

    it('refreshes on auth_token_expired then retries the original request', async () => {
        const { client } = await import('@/api/client')
        fetchSpy
            // 1) original GET → 401 expired (plain json so errorTranslator
            //    passes it through to authRefresh)
            .mockResolvedValueOnce(
                jsonResponse(401, { type: 'about:blank', title: 'Expired', status: 401, code: 'auth_token_expired' }),
            )
            // 2) the refresh POST issued by auth.refresh() → success
            .mockResolvedValueOnce(jsonResponse(200, { data: { user_id: 'u', email: 'a@b', locale: 'en', families: [] } }))
            // 3) the retried original GET → success
            .mockResolvedValueOnce(jsonResponse(200, { data: { ok: true } }))

        const { data } = await client.GET('/api/v1/relationships' as never)
        expect(data).toEqual({ data: { ok: true } })
        expect(routerReplace).not.toHaveBeenCalled()
        // original + refresh + retry = 3 fetches
        expect(fetchSpy).toHaveBeenCalledTimes(3)
    })

    it('ends the session when the retried request is still 401', async () => {
        const { client } = await import('@/api/client')
        const { ApiClientError } = await import('@/api/errors')
        fetchSpy
            .mockResolvedValueOnce(
                jsonResponse(401, { type: 'about:blank', title: 'Expired', status: 401, code: 'auth_token_expired' }),
            )
            .mockResolvedValueOnce(jsonResponse(200, { data: { user_id: 'u', email: 'a@b', locale: 'en', families: [] } }))
            // retry STILL 401 — refresh didn't actually recover the session
            .mockResolvedValueOnce(
                jsonResponse(401, { type: 'about:blank', title: 'Expired', status: 401, code: 'auth_token_expired' }),
            )

        let thrown: unknown
        try {
            await client.GET('/api/v1/relationships' as never)
        } catch (e) {
            thrown = e
        }
        expect(thrown).toBeInstanceOf(ApiClientError)
        expect(routerReplace).toHaveBeenCalledWith('/auth/sign-in')
    })
})
