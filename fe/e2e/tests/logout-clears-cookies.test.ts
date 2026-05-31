// `/auth/logout` is mounted public so the FE can drop stale HttpOnly
// cookies even when the access cookie has already expired. This e2e pins
// the round-trip: sign in, sign out, and confirm both cookies are gone
// from the browser. Without the public-logout path, this test failed
// because the cookies survived the sign-out call.

import { expect, test } from '../fixtures/console.fixture'
import { signIn } from '../page-objects/session'

test('signing out clears both the access and refresh HttpOnly cookies', async ({ browser }) => {
    // Fresh context so we own the cookie jar; isolated from other tests.
    const ctx = await browser.newContext()
    const page = await ctx.newPage()

    const stamp = Date.now()
    const email = `logout-cookies-${stamp}@example.com`
    await signIn(page, email)

    // After signIn the browser holds both cookies. We don't assert specific
    // names beyond the well-known pair because the cookie SHAPE (domain /
    // path / SameSite) is pinned by the BE integration tests; here we just
    // need them to EXIST so the post-logout disappearance is meaningful.
    const before = await ctx.cookies()
    const names = before.map((c) => c.name)
    expect(names).toContain('access')
    expect(names).toContain('refresh')

    // Sign out via the user-menu (the only logout affordance the SPA
    // exposes). The menu uses Vuetify v-menu — first open it, then click
    // the sign-out list item.
    await page.getByTestId('user-menu').click()
    await page.getByTestId('sign-out').click()
    // Auth store flips to anonymous and the router replaces to /auth/sign-in.
    await expect(page).toHaveURL(/\/auth\/sign-in$/, { timeout: 10_000 })

    // The clearing headers from `/auth/logout` set Max-Age=0 on both
    // cookies — the browser drops them. After the redirect lands we
    // re-read the jar and confirm.
    const after = await ctx.cookies()
    const afterNames = after.map((c) => c.name)
    expect(afterNames).not.toContain('access')
    expect(afterNames).not.toContain('refresh')

    await ctx.close()
})
