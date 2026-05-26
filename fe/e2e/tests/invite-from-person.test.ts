import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Magic-link sign-in. Same shape as the sibling admin-members spec — the
// helper is duplicated inline by convention so each spec stays
// self-contained.
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

/**
 * Create a person row through the FE so the e2e flow exercises real
 * UI affordances rather than reaching into the API. Returns the
 * generated person id.
 */
async function createPerson(page: Page, givenName: string, familyName: string): Promise<string> {
    await page.goto('/tree')
    // Family is pre-selected by createFamily; click the "add person" CTA.
    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill(givenName)
    await page.getByTestId('person-family-name').locator('input').fill(familyName)
    await page.getByTestId('person-submit').click()
    // The drawer flips back to view mode and renders the just-created
    // person — pull the id off the rendered tree node.
    const node = page.locator('[data-testid^="tree-node-"]').filter({ hasText: givenName }).first()
    await expect(node).toBeVisible()
    const testId = await node.getAttribute('data-testid')
    if (testId === null) throw new Error('tree node testid missing')
    return testId.replace('tree-node-', '')
}

test('admin invites a person; recipient accepts and is linked', async ({ browser }) => {
    const stamp = Date.now()
    // Owner sets up the family + creates the person to invite-as.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `invite-owner-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    await createFamily(owner, `InviteFam-${stamp}`)

    // Hannelore — `createPerson` already opens the PersonDetail drawer
    // for the newly-created row, so the invite CTA is one click away.
    await createPerson(owner, 'Hannelore', `Tester-${stamp}`)
    await clearMailpit()
    await expect(owner.getByTestId('person-detail')).toBeVisible()
    await owner.getByTestId('person-invite-cta').click()
    await expect(owner.getByTestId('person-invite-modal')).toBeVisible()

    const inviteeEmail = `hannelore-${stamp}@example.com`
    await owner.getByTestId('person-invite-email').locator('input').fill(inviteeEmail)
    await owner.getByTestId('person-invite-submit').click()
    // Toast confirms the invite landed — modal closes automatically.
    await expect(owner.getByTestId('person-invite-modal')).toBeHidden()

    // Pull the invite link from the captured email.
    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-family|Einladung zur Familie/.test(s))
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) throw new Error('invite link not in email')
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) throw new Error('invite link empty')

    // New browser context = the recipient's session, isolated from the
    // owner's cookies + localStorage so the JWT carries the invitee's
    // identity.
    const inviteeCtx = await browser.newContext()
    const invitee = await inviteeCtx.newPage()
    await signIn(invitee, inviteeEmail)
    await invitee.goto(rewriteEmailLink(inviteLink))
    // InviteAccept view POSTs /invites/accept and pushes /tree on success.
    await expect(invitee).toHaveURL(/\/tree$/)

    // The recipient's tree should mark the linked person with the
    // `current-user` class on TreeNode (set by FamilyTree when
    // `linked_user_id` matches the signed-in user).
    const myNode = invitee.locator('.tree-node.current-user').first()
    await expect(myNode).toBeVisible()
    await expect(myNode).toContainText(/Hannelore/)

    // Back on the owner's session, re-open the now-linked Hannelore drawer.
    // The invite CTA must be gone — admins shouldn't be able to re-invite a
    // person row that already maps to an account.
    await owner.goto('/tree')
    const hanneloreNode = owner.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Hannelore' }).first()
    await hanneloreNode.click()
    await expect(owner.getByTestId('person-detail')).toBeVisible()
    await expect(owner.getByTestId('person-linked-account-chip')).toBeVisible()
    await expect(owner.getByTestId('person-invite-cta')).toHaveCount(0)

    await inviteeCtx.close()
    await ownerCtx.close()
})

test('admin cancels a pending invite from /admin/invites', async ({ browser }) => {
    const stamp = Date.now()
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `invite-cancel-owner-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `CancelFam-${stamp}`)

    // Create an invite through the API (faster than the FE for setup).
    await clearMailpit()
    const inviteRes = await owner.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: `to-cancel-${stamp}@example.com`, role: 'user' },
    })
    expect(inviteRes.ok()).toBeTruthy()

    // Open /admin/invites and cancel the row.
    await owner.goto('/admin/invites')
    await expect(owner.getByTestId('admin-invites-page')).toBeVisible()
    const cancelBtn = owner.locator('button[data-testid^="admin-invites-cancel-"]').first()
    await expect(cancelBtn).toBeVisible()
    await cancelBtn.click()
    await expect(owner.getByTestId('admin-invites-confirm-dialog')).toBeVisible()
    await owner.getByTestId('admin-invites-confirm').click()

    // Row vanishes; empty state appears.
    await expect(owner.getByTestId('admin-invites-empty')).toBeVisible()

    await ownerCtx.close()
})
