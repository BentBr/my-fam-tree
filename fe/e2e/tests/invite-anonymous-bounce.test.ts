import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Magic-link sign-in shared by both contexts. Duplicated inline by
// convention so each spec stays self-contained.
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

test('anonymous invitee resumes the invite after sign-in (one click)', async ({ browser }) => {
    const stamp = Date.now()

    // Owner sets up the family + an invite for the newbie.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `bounce-owner-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `BounceFam-${stamp}`)

    const newbieEmail = `bounce-newbie-${stamp}@example.com`
    await clearMailpit()
    const inviteRes = await owner.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: newbieEmail, role: 'user' },
    })
    expect(inviteRes.ok()).toBeTruthy()

    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-family|Einladung zur Familie/.test(s))
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) throw new Error('invite link not in email')
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) throw new Error('invite link empty')

    // Brand-new context = anonymous browser. Clicking the invite link
    // should stash the token + bounce to the sign-in page (the existing
    // InviteAccept behaviour we want to preserve).
    const inviteeCtx = await browser.newContext()
    const invitee = await inviteeCtx.newPage()
    await invitee.goto(rewriteEmailLink(inviteLink))
    await expect(invitee).toHaveURL(/\/auth\/sign-in$/)
    const stashed = await invitee.evaluate(() => sessionStorage.getItem('my-family:inviteToken'))
    expect(stashed).not.toBeNull()

    // Sign in as the invitee via the magic link — single round-trip. After
    // ConsumeView resolves, the page must auto-resume the invite (the bug
    // we're fixing previously dropped the user on /tree without
    // membership).
    await clearMailpit()
    const login = new LoginPage(invitee)
    await login.signIn(newbieEmail)
    await expect(login.sent).toBeVisible()
    const signInMail = await waitForEmail((s) => /Sign in to my-family|Anmeldung bei my-family/.test(s))
    const signInMatch = signInMail.text.match(/https?:\/\/\S+\/auth\/consume\?token=\S+/)
    if (signInMatch === null) throw new Error('consume link not in email body')
    const signInLink = signInMatch[0]
    if (signInLink === undefined) throw new Error('consume link match was empty')
    await invitee.goto(rewriteEmailLink(signInLink))

    // After auto-resume, we land on /tree with the invite accepted.
    await expect(invitee).toHaveURL(/\/tree$/, { timeout: 15_000 })

    // Success toast appears (from useAcceptInvite hook's onSuccess).
    const toast = invitee.getByTestId('toast').filter({ hasText: /You joined the family|Du bist der Familie/ })
    await expect(toast).toBeVisible({ timeout: 10_000 })

    // The stash is cleared after the auto-resume.
    const afterStash = await invitee.evaluate(() => sessionStorage.getItem('my-family:inviteToken'))
    expect(afterStash).toBeNull()

    await inviteeCtx.close()
    await ownerCtx.close()
})
