import type { Page } from '@playwright/test'
import { expect, test } from '../fixtures/console.fixture'

import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Standard sign-in dance for the inviting owner.
async function signIn(page: Page, email: string): Promise<void> {
    await clearMailpit()
    const login = new LoginPage(page)
    await login.goto()
    await login.signIn(email)
    await expect(login.sent).toBeVisible()
    const mail = await waitForEmail((s) => /Sign in to my-family|Anmeldung bei my-family/.test(s))
    const match = mail.text.match(/https?:\/\/\S+\/auth\/consume\?token=\S+/)
    if (match === null) throw new Error('consume link not in email body')
    const link = match[0]
    if (link === undefined) throw new Error('consume link match was empty')
    await page.goto(rewriteEmailLink(link))
    await expect(page).toHaveURL(/\/(tree|health|families\/create|families\/pick)$/)
}

async function createFamily(page: Page, name: string): Promise<string> {
    await page.goto('/families/create')
    await page.getByTestId('family-name').locator('input').fill(name)
    await page.getByTestId('family-create-submit').click()
    await expect(page).toHaveURL(/\/tree$/)
    const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
    if (familyId === '') throw new Error('active family id missing from localStorage')
    return familyId
}

test('anonymous invite click signs the recipient in and joins the family in one step', async ({ browser }) => {
    const stamp = Date.now()

    // Owner creates a family + an invite.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    await signIn(owner, `invite-owner-${stamp}@example.com`)
    const familyId = await createFamily(owner, `InviteFam-${stamp}`)

    const inviteeEmail = `invite-anon-${stamp}@example.com`
    await clearMailpit()
    const inviteRes = await owner.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: inviteeEmail, role: 'user' },
    })
    expect(inviteRes.ok()).toBeTruthy()

    const inviteMail = await waitForEmail((s) => /invit/i.test(s))
    const inviteLinkMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteLinkMatch === null) throw new Error('invite link not in email')
    const inviteLink = inviteLinkMatch[0]
    if (inviteLink === undefined) throw new Error('invite link match was empty')

    // Recipient — brand new browser context, never logged in.
    const recipientCtx = await browser.newContext()
    const recipient = await recipientCtx.newPage()

    // Click the invite link as anonymous user. One round-trip: the BE
    // creates the user keyed on invite.email, issues a cookie, and
    // accepts the invite. No detour to /auth/sign-in.
    await recipient.goto(rewriteEmailLink(inviteLink))
    await expect(recipient).toHaveURL(/\/tree$/, { timeout: 10_000 })

    // The post-accept toast carries the "You joined the family" message.
    await expect(recipient.locator('.v-snackbar')).toContainText(/joined the family|Familie beigetreten/i, {
        timeout: 5_000,
    })

    // The recipient is signed in: the user-menu button (top-right of
    // the AppBar) carries their email as its accessible name.
    await expect(recipient.getByRole('button', { name: inviteeEmail })).toBeVisible()

    await ownerCtx.close()
    await recipientCtx.close()
})

// The email-mismatch case (signed in as alice clicking bob's invite)
// is covered by Rust integration tests in `crates/api/tests/invites_flow.rs`.
// The FE template renders `invite-mismatch` + `invite-mismatch-signout`
// per the same `validation.invite_email_mismatch` violation code; we
// trust the unit-level check on the FE side and the integration test
// on the BE side rather than running a flaky two-mailbox e2e here.
