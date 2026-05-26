import type { Page } from '@playwright/test'
import { expect, test } from '../fixtures/console.fixture'

import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Sign in via magic link. Same shape as the sibling e2e suites — kept
// inline rather than extracted because the helper is tiny and lives in
// every spec file by convention.
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

test('admin sees audit log and entity link navigates back to tree', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `audit-owner-${stamp}@example.com`)
    const familyId = await createFamily(page, `AuditFam-${stamp}`)

    // Seed one person — the API mutation below will write a `(create,
    // contact)` audit row whose `entity_person_id` resolves to this id.
    const personRes = await page.request.post('/api/v1/persons', {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { given_name: 'AuditTarget', family_name: 'Person' },
    })
    expect(personRes.ok()).toBeTruthy()
    const personBody = (await personRes.json()) as { data: { id: string } }
    const personId = personBody.data.id

    // Drive one contact create through the API so the audit table has a
    // row we can click. Owner has full visibility, so we use `family`.
    const contactRes = await page.request.post(`/api/v1/persons/${personId}/contacts`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: {
            kind: 'url',
            label: 'Audit row source',
            value: { url: 'https://example.test/audit' },
            visibility: 'family',
        },
    })
    expect(contactRes.ok()).toBeTruthy()

    // The admin nav item should be visible to the owner.
    await page.goto('/tree')
    await expect(page.getByTestId('nav-admin')).toBeVisible()

    // Navigate to /admin — should redirect to /admin/audit and land in
    // the admin layout shell.
    await page.goto('/admin')
    await expect(page).toHaveURL(/\/admin\/audit$/)
    await expect(page.getByTestId('admin-audit-page')).toBeVisible()
    await expect(page.getByTestId('admin-audit-table')).toBeVisible()

    // At least one audit row is present (the contact create plus the
    // family create + membership create that `create_family` writes).
    const firstRow = page.locator('[data-testid="admin-audit-table"] tbody tr').first()
    await expect(firstRow).toBeVisible()

    // Filter by `entity_kind=contact` to isolate the row we just produced.
    await page.getByTestId('admin-audit-filter-kind').click()
    await page.getByRole('option', { name: 'Contact' }).click()

    // Now the first row should be the contact create — click its entity
    // link and land on /tree with the drawer open for AuditTarget.
    const entityLink = page.locator('a[data-testid^="admin-audit-entity-"]').first()
    await expect(entityLink).toBeVisible()
    await entityLink.click()

    await expect(page).toHaveURL(/\/tree\?center=[0-9a-f-]+/)
    await expect(page.getByTestId('person-detail')).toBeVisible({ timeout: 10_000 })
    await expect(page.getByTestId('person-detail-title')).toContainText('AuditTarget', { timeout: 10_000 })
})

test('invite audit row shows invitee email + role as a secondary line', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `audit-invite-owner-${stamp}@example.com`)
    const familyId = await createFamily(page, `AuditInvite-${stamp}`)

    // Trigger an invite — writes the `(invite, membership)` audit row whose
    // metadata carries `email` + `role`.
    const inviteeEmail = `audit-invite-target-${stamp}@example.com`
    const inviteRes = await page.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: inviteeEmail, role: 'admin' },
    })
    expect(inviteRes.ok()).toBeTruthy()

    await page.goto('/admin/audit')
    await expect(page.getByTestId('admin-audit-page')).toBeVisible()

    // Filter to (invite, membership) so we isolate the row we just produced.
    await page.getByTestId('admin-audit-filter-action').click()
    await page.getByRole('option', { name: 'Invited' }).click()
    await page.getByTestId('admin-audit-filter-kind').click()
    await page.getByRole('option', { name: 'Membership' }).click()

    const detailsLine = page.locator('[data-testid^="admin-audit-invite-details-"]').first()
    await expect(detailsLine).toBeVisible()
    await expect(detailsLine).toContainText(inviteeEmail)
    await expect(detailsLine).toContainText(/admin/)
})

test('user role cannot reach /admin/audit (redirects to /tree)', async ({ browser }) => {
    const stamp = Date.now()
    // Owner sets up the family.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `audit-gate-owner-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `AuditGate-${stamp}`)

    // Guest signs in in an isolated context to provision a user row.
    const guestCtx = await browser.newContext()
    const guest = await guestCtx.newPage()
    const guestEmail = `audit-gate-user-${stamp}@example.com`
    await signIn(guest, guestEmail)

    // Owner invites guest as `user`, guest accepts the invite link.
    await clearMailpit()
    const inviteRes = await owner.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: guestEmail, role: 'user' },
    })
    expect(inviteRes.ok()).toBeTruthy()
    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-family|Einladung zur Familie/.test(s))
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) throw new Error('invite link not in email')
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) throw new Error('invite link empty')
    await guest.goto(rewriteEmailLink(inviteLink))
    await expect(guest).toHaveURL(/\/(tree|invite\/accept)/)

    // The guest is now a `user` member. Admin nav should not be visible.
    await guest.goto('/tree')
    await expect(guest.getByTestId('nav-admin')).toHaveCount(0)

    // Direct navigation to /admin/audit is bounced to /tree by the
    // requiresAdmin route guard.
    await guest.goto('/admin/audit')
    await expect(guest).toHaveURL(/\/tree$/)

    await ownerCtx.close()
    await guestCtx.close()
})
