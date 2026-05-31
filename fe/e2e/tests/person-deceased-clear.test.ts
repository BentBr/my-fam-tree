// Sending `death_date: null` on PATCH /persons/{id} clears the column.
// The triple-state deserializer on the BE distinguishes "absent" (preserve)
// from "explicit null" (clear); this e2e walks the UI path end-to-end so
// the "uncheck deceased" flow stays wired to the clear semantics.

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
    // Vuetify v-checkbox: the `data-testid` lands on the outer wrapper.
    // Clicking that wrapper doesn't always propagate to the `<input>`
    // (depends on the slot/label structure), so the v-model bound to
    // `deceased` would stay false. Target the inner input directly —
    // its `:checked` flip is what v-model listens to.
    await page.getByTestId('person-deceased').locator('input').check()
    // `v-if="deceased"` mounts the date field on the next tick.
    const deathDateInput = page.getByTestId('person-death-date').locator('input')
    await expect(deathDateInput).toBeVisible()
    await deathDateInput.fill('2024-06-15')
    await page.getByTestId('person-submit').click()
    // `page.click()` returns when the DOM event fires, NOT when the
    // mutation completes. Wait for the form to flip back to view mode
    // (PersonDetail's `editing` ref goes false on the `saved` emit
    // from PersonEdit, which only fires after `update.mutateAsync`
    // resolves) before reloading — otherwise the reload races the
    // in-flight PATCH and the GET returns the pre-save row.
    await expect(page.getByTestId('person-edit-button')).toBeVisible()

    // Reload + re-open the drawer to assert the date persisted server-side.
    // Use `dispatchEvent('click')` on the SVG tree node (same pattern as
    // sibling tests, e.g. contacts.test.ts): Vuetify's PersonDetail drawer
    // mounts a `v-navigation-drawer__scrim` that fades in over the tree
    // when the drawer opens, and Playwright's normal `.click()` retries
    // until "stable + no overlay" — the scrim mid-transition intercepts
    // pointer events and the click never lands. The dispatched event
    // bypasses actionability and goes straight to the node's handler.
    await page.reload()
    await page.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Werner' }).first().dispatchEvent('click')
    await expect(page.getByTestId('person-field-death-date')).toContainText('2024-06-15')

    // ----- 3. Edit again → uncheck deceased, save. -----
    await page.getByTestId('person-edit-button').click()
    // The checkbox starts checked (`deceased` ref true on mount because
    // death_date is set). Uncheck the inner input; the date field
    // unmounts AND `form.death_date` is cleared to `null` by the
    // `watch(deceased)` handler in PersonEdit.
    await page.getByTestId('person-deceased').locator('input').uncheck()
    await expect(page.getByTestId('person-death-date')).toHaveCount(0)
    await page.getByTestId('person-submit').click()
    // Same view-mode-flip sync as step 2 — wait for the save to land
    // before reloading.
    await expect(page.getByTestId('person-edit-button')).toBeVisible()

    // ----- 4. Reload and verify the BE actually NULL'd the column. -----
    // Reload + re-open is what makes this an honest round-trip: a save
    // that only updated the local form (without persisting the NULL)
    // would show "—" until refetch, then snap back to the date.
    // Same `dispatchEvent('click')` as step 2 — see comment there for
    // the v-navigation-drawer__scrim rationale.
    await page.reload()
    await page.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Werner' }).first().dispatchEvent('click')
    await expect(page.getByTestId('person-detail')).toBeVisible()
    // The view-mode list renders `—` (em-dash) for unset dates, not "2024-...".
    const deathRow = page.getByTestId('person-field-death-date')
    await expect(deathRow).toBeVisible()
    await expect(deathRow).not.toContainText('2024-06-15')
    await expect(deathRow).toContainText('—')
})
