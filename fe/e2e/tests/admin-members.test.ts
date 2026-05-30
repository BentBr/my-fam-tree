import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { signIn, createFamily } from '../page-objects/session'

/**
 * Owner invites `inviteeEmail` at `role` and the invitee accepts via
 * the magic email link. After this returns, the invitee is a member of
 * the family at the requested role.
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

test('admin can promote a user → admin from /admin/members', async ({ browser }) => {
    const stamp = Date.now()
    // Owner sets up the family.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `members-owner-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `Members-${stamp}`)

    // Promote a brand-new account from `user` → `admin`, signed in as
    // the owner (admins can also promote users → admin, but the seed
    // path is the same either way — owner is the simplest setup).
    const userCtx = await browser.newContext()
    const userPage = await userCtx.newPage()
    const userEmail = `members-user-${stamp}@example.com`
    await signIn(userPage, userEmail)
    await inviteAndAccept(owner, userPage, familyId, userEmail, 'user')

    // Owner navigates to /admin/members and promotes the user.
    await owner.goto('/admin/members')
    await expect(owner).toHaveURL(/\/admin\/members$/)
    await expect(owner.getByTestId('admin-members-table')).toBeVisible()

    // Locate the freshly-invited user's row by email cell. They're the
    // only `user`-role member — the seeded owner is the only row above
    // them so the row count is 2.
    const promoteBtn = owner.locator('button[data-testid^="admin-members-promote-"]').first()
    await expect(promoteBtn).toBeVisible()
    const testId = await promoteBtn.getAttribute('data-testid')
    if (testId === null) throw new Error('promote button missing data-testid')
    const userId = testId.replace('admin-members-promote-', '')
    await promoteBtn.click()

    // The role chip flips to Admin and the promote button disappears.
    await expect(owner.getByTestId(`admin-members-row-${userId}`)).toContainText(/Admin|admin/i)
    await expect(owner.getByTestId(`admin-members-promote-${userId}`)).toHaveCount(0)

    await ownerCtx.close()
    await userCtx.close()
})

test('admin sees no revoke button on another admin row', async ({ browser }) => {
    const stamp = Date.now()
    // Owner sets up the family and creates a second admin via invite.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `members-owner2-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `Members2-${stamp}`)

    // First admin (the one whose perspective we'll inspect later).
    const adminCtx = await browser.newContext()
    const admin = await adminCtx.newPage()
    const adminEmail = `members-admin-${stamp}@example.com`
    await signIn(admin, adminEmail)
    await inviteAndAccept(owner, admin, familyId, adminEmail, 'admin')

    // A second admin so the page has an admin row the first admin
    // shouldn't be able to revoke.
    const admin2Ctx = await browser.newContext()
    const admin2 = await admin2Ctx.newPage()
    const admin2Email = `members-admin2-${stamp}@example.com`
    await signIn(admin2, admin2Email)
    await inviteAndAccept(owner, admin2, familyId, admin2Email, 'admin')

    // First admin opens /admin/members. Wait for the table to render
    // all 3 rows before asserting on per-row buttons — the data fetch
    // can race against the table mount.
    await admin.goto('/admin/members')
    await expect(admin).toHaveURL(/\/admin\/members$/)
    await expect(admin.getByTestId('admin-members-table')).toBeVisible()
    await expect(admin.locator('tr[data-testid^="admin-members-row-"]')).toHaveCount(3)

    // Resolve the second admin's user_id from the admin2 context's
    // localStorage so we can address its row directly — no need to
    // reverse-engineer chip text. The activeFamily store doesn't track
    // user ids, so we read it from the auth /me endpoint via the
    // admin2 page (which already has a valid session).
    const admin2UserId = await admin2.evaluate(async () => {
        const res = await fetch('/api/v1/auth/me', { credentials: 'include' })
        const body = (await res.json()) as { data: { user_id: string } }
        return body.data.user_id
    })

    // The owner row never has any revoke button (matrix rule "no
    // touching the owner").
    const ownerRow = admin
        .locator('tr[data-testid^="admin-members-row-"]')
        .filter({ hasText: /Owner|Inhaber/ })
        .first()
    await expect(ownerRow.locator('button[data-testid^="admin-members-revoke-"]')).toHaveCount(0)

    // The second admin's row must have no revoke button visible to the
    // first admin — admins cannot revoke other admins.
    await expect(admin.getByTestId(`admin-members-row-${admin2UserId}`)).toBeVisible()
    await expect(admin.getByTestId(`admin-members-revoke-${admin2UserId}`)).toHaveCount(0)

    await ownerCtx.close()
    await adminCtx.close()
    await admin2Ctx.close()
})

test('owner can demote an admin → user via confirm dialog', async ({ browser }) => {
    const stamp = Date.now()
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `members-owner3-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `Members3-${stamp}`)

    const adminCtx = await browser.newContext()
    const admin = await adminCtx.newPage()
    const adminEmail = `members-admin3-${stamp}@example.com`
    await signIn(admin, adminEmail)
    await inviteAndAccept(owner, admin, familyId, adminEmail, 'admin')

    await owner.goto('/admin/members')
    await expect(owner.getByTestId('admin-members-table')).toBeVisible()

    // The newly-invited admin is the only `admin`-role row — owner can
    // demote them, which opens a confirm dialog.
    const demoteBtn = owner.locator('button[data-testid^="admin-members-demote-"]').first()
    await expect(demoteBtn).toBeVisible()
    const testId = await demoteBtn.getAttribute('data-testid')
    if (testId === null) throw new Error('demote button missing data-testid')
    const adminUserId = testId.replace('admin-members-demote-', '')
    await demoteBtn.click()

    await expect(owner.getByTestId('admin-members-confirm-dialog')).toBeVisible()
    await owner.getByTestId('admin-members-confirm').click()

    // After confirming, the row's chip should now read "User" (or
    // "Mitglied" in German). The demote button is gone; the promote
    // button reappears since the row is now `user`.
    await expect(owner.getByTestId(`admin-members-row-${adminUserId}`)).toContainText(/User|Mitglied/i)
    await expect(owner.getByTestId(`admin-members-demote-${adminUserId}`)).toHaveCount(0)

    await ownerCtx.close()
    await adminCtx.close()
})
