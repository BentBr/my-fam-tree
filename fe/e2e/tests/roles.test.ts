import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { signIn, createFamily } from '../page-objects/session'

async function createPerson(page: Page, givenName: string, familyName: string): Promise<void> {
    await page.goto('/tree')
    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill(givenName)
    await page.getByTestId('person-family-name').locator('input').fill(familyName)
    await page.getByTestId('person-submit').click()
    await expect(page.locator('[data-testid^="tree-node-"]').filter({ hasText: givenName }).first()).toBeVisible()
}

/**
 * Owner invites `inviteeEmail` at `role` (a family-level invite, NOT tied to
 * a person) and the invitee accepts via the magic link, landing as a member
 * at the requested role with no linked person row.
 */
async function inviteAndAccept(
    ownerPage: Page,
    inviteePage: Page,
    familyId: string,
    inviteeEmail: string,
    role: 'user' | 'admin',
): Promise<void> {
    await clearMailpit()
    const inviteRes = await ownerPage.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: inviteeEmail, role },
    })
    expect(inviteRes.ok()).toBeTruthy()
    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-fam-tree|Einladung zur Familie/.test(s), {
        recipient: inviteeEmail,
    })
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) throw new Error('invite link not in email')
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) throw new Error('invite link empty')
    await inviteePage.goto(rewriteEmailLink(inviteLink))
    await expect(inviteePage).toHaveURL(/\/(tree|invite\/accept)/)
}

test('user role gets a read-only person detail and no admin nav for a row they do not own', async ({ browser }) => {
    const stamp = Date.now()

    // Owner sets up the family and a person the user is NOT linked to.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `roles-owner-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `Roles-${stamp}`)
    expect(familyId).not.toBe('')
    // Short given name: the tree node label is JS-truncated, so a long
    // stamped name would never match `hasText`. The stamp lives in the
    // family name (the row is the only person in this fresh family anyway).
    await createPerson(owner, 'Otto', `Elder-${stamp}`)

    // Plain user joins the family (family-level invite, no person link).
    const userCtx = await browser.newContext()
    const user = await userCtx.newPage()
    const userEmail = `roles-user-${stamp}@example.com`
    await signIn(user, userEmail)
    await inviteAndAccept(owner, user, familyId, userEmail, 'user')

    // After accept, InviteAccept client-side-pushes to /tree with the auth +
    // active-family state already set, so we DON'T re-goto (a full reload would
    // race the store re-hydrate and bounce to /families/create). Wait for the
    // switcher to show the joined family — proof the active family is live.
    await expect(user.getByTestId('nav-drawer')).toBeVisible({ timeout: 15_000 })
    await expect(user.getByTestId('family-switcher')).toContainText(`Roles-${stamp}`, { timeout: 15_000 })

    // Admin nav entry is gated behind admin/owner — a user never sees it.
    await expect(user.getByTestId('nav-admin')).toHaveCount(0)

    // Open Otto's detail. The user owns no linked row here, so canEdit is
    // false: read-only badge shows, edit + invite affordances are absent.
    const node = user.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Otto' }).first()
    await expect(node).toBeVisible({ timeout: 15_000 })
    await node.click()
    await expect(user.getByTestId('person-detail')).toBeVisible()
    await expect(user.getByTestId('person-readonly-badge')).toBeVisible()
    await expect(user.getByTestId('person-edit-button')).toHaveCount(0)
    await expect(user.getByTestId('person-invite-cta')).toHaveCount(0)

    await userCtx.close()
    await ownerCtx.close()
})
