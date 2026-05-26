import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Magic-link sign-in. Same shape as the sibling admin-members spec —
// the helper is duplicated inline by convention so each spec stays
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
    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-family|Einladung zur Familie/.test(s))
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) throw new Error('invite link not in email')
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) throw new Error('invite link empty')
    await inviteePage.goto(rewriteEmailLink(inviteLink))
    await expect(inviteePage).toHaveURL(/\/(tree|invite\/accept)/)
}

test('owner transfers ownership to admin; both sides confirm; roles swap', async ({ browser }) => {
    const stamp = Date.now()

    // Owner sets up a fresh family + invites one admin.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `xfer-owner-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `XferFam-${stamp}`)

    const adminCtx = await browser.newContext()
    const admin = await adminCtx.newPage()
    const adminEmail = `xfer-admin-${stamp}@example.com`
    await signIn(admin, adminEmail)
    await inviteAndAccept(owner, admin, familyId, adminEmail, 'admin')

    // Owner opens /admin/members and clicks "Transfer ownership" on the
    // admin row.
    await clearMailpit()
    await owner.goto('/admin/members')
    await expect(owner.getByTestId('admin-members-table')).toBeVisible()
    const transferBtn = owner.locator('button[data-testid^="admin-members-transfer-"]').first()
    await expect(transferBtn).toBeVisible()
    const transferTestId = await transferBtn.getAttribute('data-testid')
    if (transferTestId === null) throw new Error('transfer button missing testid')
    const adminUserId = transferTestId.replace('admin-members-transfer-', '')
    await transferBtn.click()
    await owner.getByTestId('admin-members-transfer-submit').click()

    // Pending banner now visible.
    await expect(owner.getByTestId('admin-members-transfer-banner')).toBeVisible()

    // Two emails sent — one for each side.
    const ownerMail = await waitForEmail((s) => /Confirm ownership|Eigentumsübertragung bestätigen/i.test(s))
    const adminMail = await waitForEmail((s) => /offered ownership|Eigentumsübertragung für/i.test(s))
    const linkRe = /https?:\/\/\S+\/account\/owner-transfer\/confirm\?token=\S+/
    const ownerLink = ownerMail.text.match(linkRe)?.[0]
    const adminLink = adminMail.text.match(linkRe)?.[0]
    expect(ownerLink).toBeDefined()
    expect(adminLink).toBeDefined()

    // Owner clicks their link — lands on success-one state.
    await owner.goto(rewriteEmailLink(ownerLink ?? ''))
    await expect(owner.getByTestId('owner-transfer-confirm')).toBeVisible()
    await expect(owner.getByTestId('owner-transfer-success-one')).toBeVisible()

    // Admin clicks theirs — second confirm completes the swap.
    await admin.goto(rewriteEmailLink(adminLink ?? ''))
    await expect(admin.getByTestId('owner-transfer-confirm')).toBeVisible()
    await expect(admin.getByTestId('owner-transfer-success-both')).toBeVisible()

    // Refresh owner's page — `/admin/members` reads role chips from the
    // API, which is the source of truth post-swap. The owner's JWT
    // still claims `owner` on the access cookie, but the FE's
    // requires-admin gate only blocks non-admin/non-owner; the page
    // will render either way and the chips reflect DB state.
    await owner.goto('/admin/members')
    await expect(owner.getByTestId('admin-members-table')).toBeVisible()
    const ownerOwnRowId = await owner.evaluate(async () => {
        const res = await fetch('/api/v1/auth/me', { credentials: 'include' })
        const body = (await res.json()) as { data: { user_id: string } }
        return body.data.user_id
    })
    await expect(owner.getByTestId(`admin-members-row-${ownerOwnRowId}`)).toContainText(/Admin/i)
    // The promoted admin's row chip reads Owner / Inhaber.
    await expect(owner.getByTestId(`admin-members-row-${adminUserId}`)).toContainText(/Owner|Inhaber/i)

    await ownerCtx.close()
    await adminCtx.close()
})
