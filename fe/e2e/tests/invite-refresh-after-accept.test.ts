// Regression guard for the invite-accept refresh-cookie issuance bug.
//
// Symptom the user saw in prod: an invited recipient could click their
// invite link, land on /tree, and APPEAR signed in — but the next time
// the access cookie expired (or they came back later), the FE bounced
// them to /auth/sign-in with an "invalid / expired" message and they
// had to request a fresh magic link.
//
// Cause: `POST /invites/accept` only set the SHORT-lived `access`
// cookie (~15 min TTL) and skipped the refresh-token mint that
// `POST /auth/consume` does. The browser had nothing to refresh from
// once the access cookie expired.
//
// Fix (BE): invite-accept now mints + persists a refresh token via the
// shared `mint_refresh_token_for` service helper (same path
// /auth/consume takes) and sets both cookies.
//
// This test simulates the post-accept expiry by clearing JUST the
// access cookie (the same trick `refresh-after-access-cookie-gone`
// uses for the magic-link path) — the surviving refresh cookie must
// be present and valid for the FE's silent refresh to succeed.

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { createFamily, signIn } from '../page-objects/session'

test('an invite-accept mints a refresh cookie so the recipient stays signed in past access-token expiry', async ({
    browser,
}) => {
    const stamp = Date.now()

    // ----- Owner: sign in + create family + send invite. -----
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    await signIn(owner, `invite-refresh-owner-${stamp}@example.com`)
    const ownerFamilyId = await createFamily(owner, `InviteRefreshFam-${stamp}`)
    expect(ownerFamilyId).not.toEqual('')

    await clearMailpit()
    const guestEmail = `invite-refresh-guest-${stamp}@example.com`
    const inviteRes = await owner.request.post(`/api/v1/families/${ownerFamilyId}/invites`, {
        headers: { 'X-Family-Id': ownerFamilyId, 'content-type': 'application/json' },
        data: { email: guestEmail, role: 'user' },
    })
    expect(inviteRes.ok()).toBeTruthy()

    const mail = await waitForEmail((s) => /Join the .+ family on my-fam-tree|Einladung zur Familie/.test(s), {
        recipient: guestEmail,
    })
    const m = mail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (m === null) throw new Error('invite link not in email')
    const link = m[0]
    if (link === undefined) throw new Error('invite link empty')

    // ----- Guest: fresh browser context, accept the invite. -----
    const guestCtx = await browser.newContext()
    const guest = await guestCtx.newPage()
    await guest.goto(rewriteEmailLink(link))
    await expect(guest).toHaveURL(/\/tree$/, { timeout: 15_000 })

    // ----- The regression pin: BOTH cookies must be present. -----
    // Before the fix, only `access` was set and this assertion failed.
    const cookiesAfterAccept = await guestCtx.cookies()
    const namesAfterAccept = cookiesAfterAccept.map((c) => c.name)
    expect(namesAfterAccept, 'invite-accept must mint both access + refresh cookies').toEqual(
        expect.arrayContaining(['access', 'refresh']),
    )

    // ----- Simulate access-cookie expiry by dropping ONLY it. -----
    // From the browser's perspective this is identical to "TTL elapsed,
    // browser stopped sending the access cookie": the refresh cookie
    // survives because its TTL is much longer (and the FE's silent
    // refresh middleware should mint a new access cookie on the next
    // 401 from /auth/me).
    await guestCtx.clearCookies({ name: 'access' })
    const afterClear = await guestCtx.cookies()
    expect(afterClear.map((c) => c.name)).not.toContain('access')
    expect(afterClear.map((c) => c.name)).toContain('refresh')

    // ----- Trigger an authenticated request via a navigation. -----
    // /tree is family-active-gated; the silent-refresh should run
    // during the auth hydrate, mint a new access cookie, and let the
    // navigation proceed. If the refresh cookie were missing (the bug
    // this test guards), hydrate would fail and the route guard would
    // bounce the user to /auth/sign-in.
    await guest.goto('/tree')
    await expect(guest).toHaveURL(/\/tree$/, { timeout: 25_000 })
    // A fresh access cookie was minted.
    const afterRefresh = await guestCtx.cookies()
    expect(afterRefresh.map((c) => c.name)).toContain('access')
    expect(afterRefresh.map((c) => c.name)).toContain('refresh')

    await guestCtx.close()
    await ownerCtx.close()
})
