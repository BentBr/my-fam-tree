import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { signIn, createFamily } from '../page-objects/session'

async function createPerson(page: Page, givenName: string, familyName: string): Promise<string> {
    await page.goto('/tree')
    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill(givenName)
    await page.getByTestId('person-family-name').locator('input').fill(familyName)
    await page.getByTestId('person-submit').click()
    const node = page.locator('[data-testid^="tree-node-"]').filter({ hasText: givenName }).first()
    await expect(node).toBeVisible()
    const testId = await node.getAttribute('data-testid')
    if (testId === null) throw new Error('tree node testid missing')
    return testId.replace('tree-node-', '')
}

test('user role can edit their own linked person row', async ({ browser }) => {
    const stamp = Date.now()

    // Owner sets up the family + creates the person to invite-as.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `self-edit-owner-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    await createFamily(owner, `SelfEdit-${stamp}`)

    await createPerson(owner, 'Selma', `Tester-${stamp}`)
    await expect(owner.getByTestId('person-detail')).toBeVisible()
    await clearMailpit()
    await owner.getByTestId('person-invite-cta').click()
    await expect(owner.getByTestId('person-invite-modal')).toBeVisible()

    const inviteeEmail = `self-edit-target-${stamp}@example.com`
    await owner.getByTestId('person-invite-email').locator('input').fill(inviteeEmail)
    await owner.getByTestId('person-invite-submit').click()
    await expect(owner.getByTestId('person-invite-modal')).toBeHidden()

    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-fam-tree|Einladung zur Familie/.test(s))
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) throw new Error('invite link not in email')
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) throw new Error('invite link empty')

    // Recipient signs in + accepts in an isolated context.
    const inviteeCtx = await browser.newContext()
    const invitee = await inviteeCtx.newPage()
    await signIn(invitee, inviteeEmail)
    await invitee.goto(rewriteEmailLink(inviteLink))
    await expect(invitee).toHaveURL(/\/tree$/)

    // Wait for the tree query to actually render a node before asserting
    // on `.current-user`. The URL flips to /tree the moment InviteAccept
    // navigates, but the tree GET is still in flight — selecting
    // `.current-user` before the canvas paints is the race that made
    // this test flaky. The linked_user_id is written server-side during
    // accept, so once ANY node is visible the current-user class is too.
    await expect(invitee.locator('[data-testid^="tree-node-"]').first()).toBeVisible({ timeout: 15_000 })

    // Open the recipient's own person row — `.current-user` is set by
    // FamilyTree on the node whose linked_user_id matches the JWT user.
    const myNode = invitee.locator('.tree-node.current-user').first()
    await expect(myNode).toBeVisible({ timeout: 15_000 })
    await myNode.click()
    await expect(invitee.getByTestId('person-detail')).toBeVisible()
    await expect(invitee.getByTestId('person-edit-button')).toBeVisible()

    // Edit the nickname + save. The BE allows `user` role to PATCH the
    // row they're linked to; the FE gate (canEdit) is what makes the
    // button visible in the first place.
    await invitee.getByTestId('person-edit-button').click()
    const nicknameField = invitee.getByTestId('person-nickname').locator('input')
    await nicknameField.fill(`SelfNick-${stamp}`)
    await invitee.getByTestId('person-submit').click()

    // Drawer flips back to view mode + the nickname is shown. The flip
    // waits on the PATCH + the drawer's re-render, so give it headroom.
    await expect(invitee.getByTestId('person-field-nickname')).toContainText(`SelfNick-${stamp}`, { timeout: 15_000 })

    // Reload to confirm the value persisted server-side. Re-navigate
    // directly rather than `reload()` — a fresh route lands without the
    // detail drawer scrim that can intercept the click on mobile-shaped
    // viewports.
    await invitee.goto('/tree')
    await expect(invitee.locator('[data-testid^="tree-node-"]').first()).toBeVisible({ timeout: 15_000 })
    const myNodeAfter = invitee.locator('.tree-node.current-user').first()
    await expect(myNodeAfter).toBeVisible({ timeout: 15_000 })
    await myNodeAfter.click()
    await expect(invitee.getByTestId('person-field-nickname')).toContainText(`SelfNick-${stamp}`, { timeout: 15_000 })

    await inviteeCtx.close()
    await ownerCtx.close()
})
