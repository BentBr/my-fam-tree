// Self-claim: an admin/owner with an account creates a person row, then
// clicks "This is me" in the drawer to link that row to themselves —
// no email round-trip. Verifies the BE POST /persons/{id}/claim wiring
// and the FE PersonDetail "claim CTA → linked-account chip" path.

import { expect, test } from '../fixtures/console.fixture'
import { signIn, createFamily } from '../page-objects/session'

test('owner can claim a person row as themselves without an invite email', async ({ page }) => {
    const stamp = Date.now()
    const ownerEmail = `claim-owner-${stamp}@example.com`
    await signIn(page, ownerEmail)
    await createFamily(page, `ClaimFam-${stamp}`)

    // Create a person via the FE — the drawer opens on the new row.
    await page.goto('/tree')
    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill('Werner')
    await page.getByTestId('person-family-name').locator('input').fill(`Owner-${stamp}`)
    await page.getByTestId('person-submit').click()

    // PersonDetail mounts on the newly-created row. Before the claim,
    // neither chip is rendered — the row is unlinked.
    await expect(page.getByTestId('person-detail')).toBeVisible()
    await expect(page.getByTestId('person-its-you-chip')).toHaveCount(0)
    await expect(page.getByTestId('person-linked-account-chip')).toHaveCount(0)

    // The claim CTA is visible because the viewer is the owner of this
    // family AND the row is unlinked. Click it.
    const claimCta = page.getByTestId('person-claim-cta')
    await expect(claimCta).toBeVisible()
    await claimCta.click()

    // After the mutation resolves the cached person GET is invalidated;
    // the drawer re-renders with `person.linked_user_id === auth.user.id`,
    // which flips the chip from "Has account" to "It's you!" via the
    // `isMe` computed in PersonDetail.
    await expect(page.getByTestId('person-its-you-chip')).toBeVisible({ timeout: 10_000 })
    // The claim CTA is gone (the conditional is "unlinked only").
    await expect(claimCta).toHaveCount(0)

    // A second click on the same row in a fresh view (re-open) MUST NOT
    // re-offer the CTA — the chip persists and the action row only shows
    // edit / invite stays hidden (linked rows can't be re-invited).
    await page.goto('/tree')
    const node = page.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Werner' }).first()
    await node.click()
    await expect(page.getByTestId('person-its-you-chip')).toBeVisible()
    await expect(page.getByTestId('person-claim-cta')).toHaveCount(0)
    await expect(page.getByTestId('person-invite-cta')).toHaveCount(0)
})
