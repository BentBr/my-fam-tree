import { expect, type Page, test } from '@playwright/test'

import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// T7 e2e coverage: every signed-in user is either viewing an auto-selected
// family or being funneled to /families/create. Mirrors the inline signIn
// helpers in the other e2e files.
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
    // Wait for ConsumeView's POST + redirect to settle before any further
    // navigation. A guard-driven landing on /families/create (no families),
    // /families/pick (multi-family), /tree (single-family auto-select), or
    // /health is all acceptable here — callers narrow as needed.
    await expect(page).toHaveURL(/\/(tree|health|families\/create|families\/pick)$/)
}

async function createFamily(page: Page, name: string): Promise<void> {
    await page.goto('/families/create')
    await page.getByTestId('family-name').locator('input').fill(name)
    await page.getByTestId('family-create-submit').click()
    await expect(page).toHaveURL(/\/health$/)
}

test.describe('FE family flow', () => {
    test('fresh user lands on /families/create', async ({ page }) => {
        const stamp = Date.now()
        await signIn(page, `fresh-${stamp}@example.com`)
        // No families yet — the second router guard funnels straight to the
        // create view rather than the picker.
        await expect(page).toHaveURL(/\/families\/create$/)
    })

    test('family switcher is always visible with "create new" even when empty', async ({ page }) => {
        const stamp = Date.now()
        await signIn(page, `empty-switcher-${stamp}@example.com`)
        await expect(page).toHaveURL(/\/families\/create$/)
        // T7: the switcher renders even with zero families; only the "create new"
        // entry is available. Open it and pick that entry — it routes to /families/create
        // (no-op if already there, but the click should not error).
        await expect(page.getByTestId('family-switcher')).toBeVisible()
        await page.getByTestId('family-switcher').click()
        await page.getByRole('option', { name: /Create new family|Neue Familie anlegen/ }).click()
        await expect(page).toHaveURL(/\/families\/create$/)
    })

    test('single-family user is auto-selected and lands on /tree directly', async ({ page }) => {
        const stamp = Date.now()
        await signIn(page, `auto-${stamp}@example.com`)
        await createFamily(page, `Auto-${stamp}`)
        // From /health, navigate to /tree — the single-family guard branch
        // must auto-select and proceed; no /families/pick stop in the middle.
        await page.goto('/tree')
        await expect(page).toHaveURL(/\/tree$/)
        // The active family appears in the switcher's combobox value.
        const switcherInput = page.getByTestId('family-switcher').locator('input').first()
        await expect(switcherInput).toHaveValue(`Auto-${stamp}`)
    })

    test('/tree without an active family does not error — guard redirects', async ({ page }) => {
        const stamp = Date.now()
        await signIn(page, `redirect-${stamp}@example.com`)
        // Brand-new user with zero families: the consume redirect lands on
        // /families/create. Now try jumping straight to /tree — the family
        // guard must redirect (not let the tree query 500 on a missing
        // family-id header).
        await page.goto('/tree')
        await expect(page).toHaveURL(/\/families\/create$/)
        await expect(page.getByTestId('family-name')).toBeVisible()
    })
})
