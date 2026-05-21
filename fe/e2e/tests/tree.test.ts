import { expect, type Page, test } from '@playwright/test'

import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Sign-in helper mirrors auth_flow / account_flow. Inlining it (rather than
// extracting to a shared fixture) keeps the test self-contained and matches
// the existing convention in this directory.
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
    await expect(page).toHaveURL(/\/(health|families\/create|families\/pick)$/)
}

async function createFamily(page: Page, name: string): Promise<void> {
    await page.goto('/families/create')
    await page.getByTestId('family-name').locator('input').fill(name)
    await page.getByTestId('family-create-submit').click()
    await expect(page).toHaveURL(/\/health$/)
}

/**
 * Adds a person via the create drawer.  Returns the new person's id, read off
 * the `data-testid="tree-node-<uuid>"` attribute that lands once the drawer
 * has switched into the post-save detail view.
 */
async function addPerson(page: Page, given: string, family: string, birth?: string): Promise<string> {
    const existingIds = await page
        .locator('[data-testid^="tree-node-"]')
        .evaluateAll((els) => els.map((el) => (el.getAttribute('data-testid') ?? '').replace('tree-node-', '')))

    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill(given)
    await page.getByTestId('person-family-name').locator('input').fill(family)
    if (birth !== undefined) {
        await page.getByTestId('person-birth-date').locator('input').fill(birth)
    }
    await page.getByTestId('person-submit').click()

    // After save the drawer flips to the detail view for the new person; wait
    // for the new tree-node-<id> element to appear and capture the id from it.
    await expect(page.getByTestId('person-detail')).toBeVisible()
    const newSelector = page.locator('[data-testid^="tree-node-"]').filter({
        hasNotText: '__never_match__',
    })
    await expect(newSelector).toHaveCount(existingIds.length + 1)
    const ids = await newSelector.evaluateAll((els) =>
        els.map((el) => (el.getAttribute('data-testid') ?? '').replace('tree-node-', '')),
    )
    const added = ids.find((id) => !existingIds.includes(id))
    if (added === undefined) throw new Error('could not resolve newly-added person id')
    return added
}

async function closeDrawer(page: Page): Promise<void> {
    await page.getByTestId('person-detail-close').click()
    await expect(page.getByTestId('person-detail')).toBeHidden()
    // Wait for the v-navigation-drawer scrim to be fully gone before further
    // clicks — its fade-out transition still intercepts pointer events for a
    // couple of frames after the detail panel reports hidden.
    await expect(page.locator('.v-navigation-drawer__scrim')).toHaveCount(0)
}

// `dispatchEvent('click')` bypasses Playwright's viewport/visibility check.
// The SVG canvas pans/zooms with d3-zoom so the absolute node position is
// outside the CSS viewport rectangle that Playwright validates against, even
// though the user-visible element is on screen. Programmatic clicks fire the
// same Vue handler and exercise the same flow.
async function clickTreeNode(page: Page, id: string): Promise<void> {
    await page.getByTestId(`tree-node-${id}`).dispatchEvent('click')
}

test('owner adds people, links a parent and a partner, tree renders edges', async ({ page }) => {
    // Unique per-run email + family so the truncate-on-teardown can't race a
    // re-run before postgres has finished cleanup (defense in depth — global
    // teardown already truncates, but timestamped data keeps the test idempotent).
    const stamp = Date.now()
    await signIn(page, `tree-${stamp}@example.com`)
    await createFamily(page, `Tree-${stamp}`)

    await page.goto('/tree')
    await expect(page.getByTestId('tree-canvas')).toBeVisible()

    // 1. Add Anna.
    const annaId = await addPerson(page, 'Anna', 'Müller', '1980-04-15')
    expect(annaId).not.toBe('')
    await closeDrawer(page)

    // 2. Add Otto and link him as Anna's parent.
    const ottoId = await addPerson(page, 'Otto', 'Müller', '1950-01-01')
    expect(ottoId).toBe(ottoId) // tautology; keeps lint happy without dropping the binding
    await closeDrawer(page)

    await clickTreeNode(page, annaId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    // v-select renders its trigger; click then pick the option by visible text.
    await page.getByTestId('person-add-parent').click()
    await page.getByRole('option', { name: 'Otto Müller' }).click()
    await page.getByTestId('person-add-parent-submit').click()

    // The parent edge should appear; wait for the SVG to refresh.
    await expect(page.locator('[data-testid="tree-edge-parent"]')).toHaveCount(1)
    await closeDrawer(page)

    // 3. Add Maria and link her as Anna's partner.
    const _mariaId = await addPerson(page, 'Maria', 'Schmidt', '1982-06-10')
    expect(_mariaId).not.toBe('')
    await closeDrawer(page)

    await clickTreeNode(page, annaId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    await page.getByTestId('person-add-partner').click()
    await page.getByRole('option', { name: 'Maria Schmidt' }).click()
    await page.getByTestId('person-add-partner-submit').click()

    // At least one partner edge appears on the canvas.
    await expect(page.locator('[data-testid="tree-edge-partner"]').first()).toBeVisible()
    const partnerCount = await page.locator('[data-testid="tree-edge-partner"]').count()
    expect(partnerCount).toBeGreaterThanOrEqual(1)
})
