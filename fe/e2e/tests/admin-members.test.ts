import { expect, test } from '../fixtures/console.fixture'
import { inviteAndAccept } from '../page-objects/invites'
import { signedInContext, createFamily } from '../page-objects/session'
import { uniqueStamp } from '../page-objects/unique'

test('admin can promote a user → admin from /admin/members', async ({ browser }) => {
    const stamp = uniqueStamp()
    // Owner sets up the family.
    const ownerEmail = `members-owner-${stamp}@example.com`
    const { context: ownerCtx, page: owner } = await signedInContext(browser, ownerEmail)
    const familyId = await createFamily(owner, `Members-${stamp}`)

    // Promote a brand-new account from `user` → `admin`, signed in as
    // the owner (admins can also promote users → admin, but the seed
    // path is the same either way — owner is the simplest setup).
    const userEmail = `members-user-${stamp}@example.com`
    const { context: userCtx, page: userPage } = await signedInContext(browser, userEmail)
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
    const stamp = uniqueStamp()
    // Owner sets up the family and creates a second admin via invite.
    const ownerEmail = `members-owner2-${stamp}@example.com`
    const { context: ownerCtx, page: owner } = await signedInContext(browser, ownerEmail)
    const familyId = await createFamily(owner, `Members2-${stamp}`)

    // First admin (the one whose perspective we'll inspect later).
    const adminEmail = `members-admin-${stamp}@example.com`
    const { context: adminCtx, page: admin } = await signedInContext(browser, adminEmail)
    await inviteAndAccept(owner, admin, familyId, adminEmail, 'admin')

    // A second admin so the page has an admin row the first admin
    // shouldn't be able to revoke.
    const admin2Email = `members-admin2-${stamp}@example.com`
    const { context: admin2Ctx, page: admin2 } = await signedInContext(browser, admin2Email)
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
    const stamp = uniqueStamp()
    const ownerEmail = `members-owner3-${stamp}@example.com`
    const { context: ownerCtx, page: owner } = await signedInContext(browser, ownerEmail)
    const familyId = await createFamily(owner, `Members3-${stamp}`)

    const adminEmail = `members-admin3-${stamp}@example.com`
    const { context: adminCtx, page: admin } = await signedInContext(browser, adminEmail)
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
