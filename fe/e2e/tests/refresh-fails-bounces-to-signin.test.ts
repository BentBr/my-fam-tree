// Scenario: the access cookie is gone AND the refresh cookie is gone (or
// invalid), so the silent-refresh path CANNOT recover the session. The FE
// must surface that as a clean bounce to /auth/sign-in — not hang on the
// failed page, not swallow the 401.
//
// This is the negative twin of `refresh-after-access-cookie-gone`: there the
// refresh succeeds and the user stays put; here it fails and the user is
// sent to sign-in. Together they pin both branches of the `authRefresh`
// middleware (client.ts): 401 → POST /auth/refresh → {retry | endSession}.
//
// Mechanism: after sign-in the Pinia auth store is still `authenticated`
// in memory (so the router guard lets us onto /account), but the cookie jar
// is empty. The page-mount API call 401s, `authRefresh` POSTs /auth/refresh,
// that ALSO 401s (no refresh cookie), so `endSession()` runs → router
// replace to /auth/sign-in. Covered at the unit level by client.test.ts
// ("ends the session when the retried request is still 401"); this is the
// full-browser proof.

import { expect, test } from '../fixtures/console.fixture'
import { signedInContext, createFamily } from '../page-objects/session'
import { uniqueStamp } from '../page-objects/unique'

test('a failed refresh (both cookies gone) bounces the user to sign-in', async ({ browser }) => {
    const stamp = uniqueStamp()
    const email = `refresh-fails-${stamp}@example.com`
    const { context: ctx, page } = await signedInContext(browser, email)
    await createFamily(page, `RefreshFails-${stamp}`)
    await expect(page).toHaveURL(/\/tree$/, { timeout: 15_000 })

    // Drop BOTH auth cookies — now nothing can recover the session.
    const before = await ctx.cookies()
    expect(before.map((c) => c.name)).toEqual(expect.arrayContaining(['access', 'refresh']))
    await ctx.clearCookies({ name: 'access' })
    await ctx.clearCookies({ name: 'refresh' })
    const namesAfterClear = (await ctx.cookies()).map((c) => c.name)
    expect(namesAfterClear).not.toContain('access')
    expect(namesAfterClear).not.toContain('refresh')

    // Trigger an authenticated request. The store still thinks it's signed
    // in (so the guard admits /account), but GET /users/me 401s → refresh
    // is attempted → refresh 401s (no cookie) → endSession → sign-in.
    await page.goto('/account')
    // Same slow auth-chain budget as the sibling success-path test (the
    // 401 → /auth/refresh 401 → endSession hop is CI cold-start sensitive).
    await expect(page).toHaveURL(/\/auth\/sign-in$/, { timeout: 25_000 })
    // The account card must NOT have rendered — the session is gone.
    await expect(page.getByTestId('account-card')).toHaveCount(0)

    await ctx.close()
})
