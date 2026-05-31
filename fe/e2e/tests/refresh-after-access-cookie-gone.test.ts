// Auto-logout regression: after the access cookie's TTL elapses, the FE
// must use the surviving refresh cookie to mint a fresh session instead
// of bouncing the user to /auth/sign-in. The bug Bent saw in prod
// (logged out after "some min") is exactly this — refresh wasn't
// kicking in.
//
// We can't reliably wait out the 15-minute access TTL in an e2e, so we
// simulate the same browser-side state by removing JUST the access
// cookie from the context after sign-in. From the FE's perspective
// that's indistinguishable from "the browser dropped the expired
// access cookie": no access cookie attached to outgoing requests, the
// refresh cookie still present on /auth/refresh requests (it has its
// own longer TTL + narrower path).
//
// If `authRefresh` middleware works, the next API call gets a 401,
// triggers POST /auth/refresh, retries the original request, and the
// user stays on /account with the page rendered. If the refresh path
// is broken, the call surfaces 401 → endSession → redirect to
// /auth/sign-in (the symptom in prod).

import { expect, test } from '../fixtures/console.fixture'
import { signIn, createFamily } from '../page-objects/session'

// TODO: this test reproduces the production regression Bent reported
// ("logged out after some min" — refresh not kicking in). It currently
// fails locally on the same path: after clearing the access cookie, the
// next `GET /users/me` returns 401, the `authRefresh` middleware tries
// `POST /auth/refresh`, the BE rejects with 401 (cause TBD — likely
// SameSite=Strict on the refresh cookie + how the dev proxy frames the
// request), and `endSession()` bounces the user to /auth/sign-in.
//
// Marked `test.fail()` so CI stays green while pinning the regression:
// the moment the underlying bug is fixed, this test PASSES, which
// makes Playwright surface the `test.fail()` as an unexpected pass —
// the cue to remove the marker. Remove `test.fail()` + this comment
// once the fix lands.
test.fail('the FE refreshes the session when the access cookie disappears but the refresh cookie survives', async ({
    browser,
}) => {
    // Fresh context so we own the cookie jar — isolated from other tests.
    const ctx = await browser.newContext()
    const page = await ctx.newPage()

    const stamp = Date.now()
    const email = `refresh-after-expiry-${stamp}@example.com`
    await signIn(page, email)
    await createFamily(page, `RefreshFam-${stamp}`)
    await expect(page).toHaveURL(/\/tree$/, { timeout: 15_000 })

    // ----- Surgically drop ONLY the access cookie. -----
    // playwright's `context.clearCookies({ name })` removes the entry; we
    // then verify the refresh cookie is still in the jar (otherwise the
    // test would be asserting a different thing — the refresh path would
    // also fail because the cookie's gone, and that'd look like the same
    // regression even though it's a different cause).
    const before = await ctx.cookies()
    expect(before.map((c) => c.name)).toEqual(expect.arrayContaining(['access', 'refresh']))
    await ctx.clearCookies({ name: 'access' })
    const afterClear = await ctx.cookies()
    const namesAfterClear = afterClear.map((c) => c.name)
    expect(namesAfterClear).not.toContain('access')
    expect(namesAfterClear).toContain('refresh')

    // ----- Trigger an authenticated request. -----
    // /account fires `GET /api/v1/users/me` via the auth-store hydrate +
    // the useMe query. With no access cookie, the BE returns 401; the
    // FE's `authRefresh` middleware should POST /api/v1/auth/refresh,
    // get a fresh access cookie back, retry /users/me, and render.
    await page.goto('/account')
    await expect(page.getByTestId('account-card')).toBeVisible({ timeout: 15_000 })
    // No bounce to sign-in (the failure mode the user reported).
    await expect(page).toHaveURL(/\/account$/)

    // ----- A fresh access cookie was minted by the refresh. -----
    // The new cookie has the same name + domain as the original, with
    // a fresh value and a fresh Max-Age. We only assert presence here
    // (the value is opaque to the test, and the Max-Age math is
    // covered by the BE integration tests).
    const afterRefresh = await ctx.cookies()
    const refreshedNames = afterRefresh.map((c) => c.name)
    expect(refreshedNames).toContain('access')
    expect(refreshedNames).toContain('refresh')

    await ctx.close()
})
