// PATCH /persons/{id} regression: clearing `death_date` from the UI must
// actually clear the column (not silently preserve the existing date).
//
// Previously a plain `Option<NaiveDate>` on the BE collapsed "field
// absent" and "field explicitly null" into the same `None`, so the
// "uncheck deceased" checkbox had no effect — the date stuck around in
// the DB and re-appeared on the next refetch. The triple-state
// deserializer on the server fixes that; this e2e walks the UI path
// end-to-end to lock the behaviour in.

import { expect, test } from '../fixtures/console.fixture'
import { signIn, createFamily } from '../page-objects/session'

test('uncheck deceased + save actually clears death_date round-trip', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `deceased-clear-${stamp}@example.com`)
    await createFamily(page, `DeceasedFam-${stamp}`)

    // ----- 1. Create the person via the FE. The drawer opens on the new row. -----
    await page.goto('/tree')
    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill('Werner')
    await page.getByTestId('person-family-name').locator('input').fill(`Tester-${stamp}`)
    await page.getByTestId('person-submit').click()
    await expect(page.getByTestId('person-detail')).toBeVisible()

    // ----- 2. Edit → mark deceased, fill date, save. -----
    await page.getByTestId('person-edit-button').click()
    await page.getByTestId('person-deceased').click()
    // The `<v-checkbox>` toggles via the underlying input; click resolves
    // through Vuetify's label wrapper. After the click the `deceased` ref
    // flips true and the date field mounts.
    const deathDateInput = page.getByTestId('person-death-date').locator('input')
    await expect(deathDateInput).toBeVisible()
    await deathDateInput.fill('2024-06-15')
    await page.getByTestId('person-submit').click()

    // Drawer flips back to view-mode; assert the date persisted server-side
    // by reloading and re-opening the detail.
    await page.reload()
    const node = page.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Werner' }).first()
    await node.click()
    await expect(page.getByTestId('person-field-death-date')).toContainText('2024-06-15')

    // ----- 3. Edit again → uncheck deceased, save. -----
    await page.getByTestId('person-edit-button').click()
    // The checkbox starts checked (`deceased` ref true from initial load
    // because death_date is set). One click unchecks; the date field
    // unmounts AND `form.death_date` is cleared to `null` by the
    // `watch(deceased)` handler in PersonEdit.
    await page.getByTestId('person-deceased').click()
    await expect(page.getByTestId('person-death-date')).toHaveCount(0)
    await page.getByTestId('person-submit').click()

    // ----- 4. Reload and verify the BE actually NULL'd the column. -----
    // Pre-fix this was the failure: the UI looked clean post-save (the
    // form was reset and re-fetched) but the DB still held the date,
    // and the next refetch put it back.
    await page.reload()
    await page.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Werner' }).first().click()
    await expect(page.getByTestId('person-detail')).toBeVisible()
    // The view-mode list renders `—` (em-dash) for unset dates, not "2024-...".
    const deathRow = page.getByTestId('person-field-death-date')
    await expect(deathRow).toBeVisible()
    await expect(deathRow).not.toContainText('2024-06-15')
    await expect(deathRow).toContainText('—')
})
