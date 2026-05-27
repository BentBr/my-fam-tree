import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { signIn } from '../page-objects/session'

test('owner signs in, creates family, invites a guest, guest joins', async ({ browser }) => {
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()

    // 1. Owner signs in (no families yet).
    await signIn(owner, 'owner@example.com')

    // The family guard should bounce the owner to /families/create since
    // they have no families. Navigate explicitly to make the intent clear
    // and tolerate either landing spot.
    await owner.goto('/families/create')
    await owner.getByTestId('family-name').locator('input').fill('Müller')
    await owner.getByTestId('family-create-submit').click()
    await expect(owner).toHaveURL(/\/tree$/)
    await expect(owner.getByTestId('family-switcher')).toBeVisible()

    // 2. Owner invites a guest via REST. The FE has no invite UI in 1b.
    const families = await owner.evaluate(async () => {
        const r = await fetch('/api/v1/families/me', { credentials: 'include' })
        const body = (await r.json()) as { data: { families: Array<{ id: string }> } }
        return body.data.families
    })
    const familyId = families[0]?.id
    if (familyId === undefined) {
        throw new Error('owner has no families after create')
    }
    await clearMailpit()
    await owner.evaluate(async (id) => {
        await fetch(`/api/v1/families/${id}/invites`, {
            method: 'POST',
            credentials: 'include',
            headers: { 'content-type': 'application/json', 'x-family-id': id },
            body: JSON.stringify({ email: 'guest@example.com', role: 'user' }),
        })
    }, familyId)

    // 3. Pull the invite link out of mailpit BEFORE the guest signs in
    //    (guest's sign-in will clear and replace the latest message).
    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-family|Einladung zur Familie/.test(s))
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) {
        throw new Error('invite link not in email body')
    }
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) {
        throw new Error('invite link match was empty')
    }

    // 4. Guest signs in in a separate browser context (independent cookies).
    const guestCtx = await browser.newContext()
    const guest = await guestCtx.newPage()
    await signIn(guest, 'guest@example.com')

    // 5. Guest follows the invite link. InviteAccept consumes it (guest is
    //    already authenticated) and the single-family auto-select then sends
    //    them straight to /tree.
    await guest.goto(rewriteEmailLink(inviteLink))
    await expect(guest).toHaveURL(/\/tree$/)
    await expect(guest.getByTestId('family-switcher')).toBeVisible()

    await ownerCtx.close()
    await guestCtx.close()
})
