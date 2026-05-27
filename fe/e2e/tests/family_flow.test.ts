import { expect, test } from '../fixtures/console.fixture'
import { signIn, createFamily } from '../page-objects/session'

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
