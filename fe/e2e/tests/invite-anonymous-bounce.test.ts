import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { signIn, createFamily } from '../page-objects/session'

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
