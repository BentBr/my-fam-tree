import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { signIn, createFamily } from '../page-objects/session'

const TREE_NODE_UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

async function listTreeNodeIds(page: Page): Promise<string[]> {
    const raw = await page
        .locator('[data-testid^="tree-node-"]')
        .evaluateAll((els) => els.map((el) => (el.getAttribute('data-testid') ?? '').replace('tree-node-', '')))
    return raw.filter((id) => TREE_NODE_UUID_RE.test(id))
}

async function addPerson(page: Page, given: string, family: string, birth?: string): Promise<string> {
    const existingIds = await listTreeNodeIds(page)
    const expectedAfter = existingIds.length + 1

    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill(given)
    await page.getByTestId('person-family-name').locator('input').fill(family)
    if (birth !== undefined) {
        await page.getByTestId('person-birth-date').locator('input').fill(birth)
    }
    await page.getByTestId('person-submit').click()

    await expect(page.getByTestId('person-detail')).toBeVisible()
    await expect.poll(async () => (await listTreeNodeIds(page)).length, { timeout: 10_000 }).toBe(expectedAfter)
    const ids = await listTreeNodeIds(page)
    const added = ids.find((id) => !existingIds.includes(id))
    if (added === undefined) throw new Error('could not resolve newly-added person id')
    return added
}

async function closeDrawer(page: Page): Promise<void> {
    await page.getByTestId('person-detail-close').click()
    await expect(page.getByTestId('person-detail')).toBeHidden()
    await expect(page.locator('.v-navigation-drawer__scrim')).toHaveCount(0)
}

async function clickTreeNode(page: Page, id: string): Promise<void> {
    // The SVG canvas pans/zooms with d3-zoom so the absolute node position
    // is outside the CSS viewport rectangle Playwright validates against.
    // Programmatic clicks fire the same Vue handler.
    await page.getByTestId(`tree-node-${id}`).dispatchEvent('click')
}

async function linkParentByName(page: Page, childId: string, parentName: string): Promise<void> {
    await clickTreeNode(page, childId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    // Open the Parents panel — the v-select for "add parent" lives inside it.
    await page.getByTestId('relations-parents').click()
    await page.getByTestId('person-add-parent').click()
    await page.getByRole('option', { name: parentName }).click()
    await page.getByTestId('person-add-parent-submit').click()
}

async function linkMarriageByName(page: Page, personId: string, partnerName: string): Promise<void> {
    await clickTreeNode(page, personId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    await page.getByTestId('relations-partners').click()
    await page.getByTestId('person-add-partner').click()
    await page.getByRole('option', { name: partnerName }).click()
    await page.getByTestId('person-add-partner-kind').click()
    await page.getByRole('option', { name: /Marriage|Ehe/ }).click()
    await page.getByTestId('person-add-partner-submit').click()
}

// ---------------------------------------------------------------------------
// Admin/owner: full edit path. Builds Klaus + Otto + Hannelore + Anna,
// wires up Klaus's parents and the Klaus↔Anna marriage, then exercises the
// PersonDetail drawer: full PersonView fields, Parents panel closed by
// default, "End partnership" pre-fills today + divorce, save → tree
// payload reflects the end_reason.
// ---------------------------------------------------------------------------

test('owner sees full data + collapsible relations and ends a partnership inline', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `pd-owner-${stamp}@example.com`)
    await createFamily(page, `PDOwner-${stamp}`)
    await page.goto('/tree')

    // Build the graph: Klaus (G2) + Otto/Hannelore (G1, parents) + Anna (G2, partner).
    const klausId = await addPerson(page, 'Klaus', 'Müller', '1965-04-22')
    await closeDrawer(page)
    await addPerson(page, 'Otto', 'Müller', '1935-03-12')
    await closeDrawer(page)
    await addPerson(page, 'Hannelore', 'Müller', '1938-07-23')
    await closeDrawer(page)
    await addPerson(page, 'Anna', 'Müller', '1968-08-11')
    await closeDrawer(page)

    await linkParentByName(page, klausId, 'Otto Müller')
    await expect(page.locator('[data-testid="tree-edge-parent"]')).toHaveCount(1)
    await closeDrawer(page)
    await linkParentByName(page, klausId, 'Hannelore Müller')
    await expect(page.locator('[data-testid="tree-edge-parent"]')).toHaveCount(2)
    await closeDrawer(page)
    await linkMarriageByName(page, klausId, 'Anna Müller')
    await expect(page.locator('[data-testid="tree-edge-partner"]').first()).toBeVisible()
    await closeDrawer(page)

    // Open Klaus's drawer. The Parents panel is rendered closed by default
    // (the expansion-panel-text content is hidden until the title is clicked).
    // We assert the panel + its title text are visible, then expand and
    // verify both parents land in the relation rows.
    await clickTreeNode(page, klausId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    await expect(page.getByTestId('person-detail-title')).toContainText('Klaus Müller')

    // PersonView fields — every field rendered (some empty as "—").
    await expect(page.getByTestId('person-field-given-name')).toContainText('Klaus')
    await expect(page.getByTestId('person-field-family-name')).toContainText('Müller')
    await expect(page.getByTestId('person-field-birth-date')).toContainText('1965-04-22')

    // Parents panel: rendered, count chip = 2.
    const parentsPanel = page.getByTestId('relations-parents')
    await expect(parentsPanel).toBeVisible()
    await expect(parentsPanel).toContainText('2')

    // Closed by default — the relation rows live inside v-expansion-panel-text
    // which Vuetify keeps `display:none` until the title is clicked.
    // Expand by clicking the title and assert both parents render.
    await parentsPanel.click()
    await expect(parentsPanel).toContainText('Otto Müller')
    await expect(parentsPanel).toContainText('Hannelore Müller')

    // Partners panel: end the Klaus↔Anna partnership via the "End partnership"
    // button. The button pre-fills the ended_on field with today's date
    // (YYYY-MM-DD, local time) and end_reason = 'divorce'.
    const partnersPanel = page.getByTestId('relations-partners')
    await partnersPanel.click()
    await expect(partnersPanel).toContainText('Anna Müller')

    // The partnership id is generated server-side; resolve it off the row's
    // data-testid prefix `relation-partner-<uuid>`.
    const partnerRowId = await partnersPanel
        .locator('[data-testid^="relation-partner-"]')
        .first()
        .evaluate((el) => (el.getAttribute('data-testid') ?? '').replace('relation-partner-', ''))
    expect(partnerRowId).toMatch(TREE_NODE_UUID_RE)

    await page.getByTestId(`relation-partner-end-${partnerRowId}`).click()

    // The end-date input is now populated with today's date (we don't
    // hardcode it; just assert the format).
    const endedOnInput = page.getByTestId(`relation-partner-ended-on-${partnerRowId}`).locator('input')
    await expect(endedOnInput).toHaveValue(/^\d{4}-\d{2}-\d{2}$/)

    // The end_reason select is shown; the inner v-select stub renders the
    // current value in its trigger. We assert the v-select trigger contains
    // "Divorce" / "Scheidung".
    const endReasonSelect = page.getByTestId(`relation-partner-end-reason-${partnerRowId}`)
    await expect(endReasonSelect).toContainText(/Divorce|Scheidung/)

    // Save: triggers PATCH /partnerships/{id}. Expect the success toast.
    await page.getByTestId(`relation-partner-save-${partnerRowId}`).click()
    const toast = page.getByTestId('toast').first()
    await expect(toast).toBeVisible()

    // Tree refetch lands a tick or two after the toast; close + reopen the
    // drawer so the next render reads the freshly-fetched partnership row
    // (with its new `ended_on`), then expand the Partners panel again.
    await closeDrawer(page)
    await clickTreeNode(page, klausId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    await page.getByTestId('relations-partners').click()
    const endedChip = page.getByTestId(`relation-partner-ended-${partnerRowId}`)
    await expect(endedChip).toBeVisible({ timeout: 10_000 })
})

// ---------------------------------------------------------------------------
// Non-admin user: read-only path. Owner builds the graph + invites a
// 'user'-role member, who then opens a person's drawer and confirms:
//  • the "Read-only" badge is present
//  • no Save / Edit / Delete buttons render
//  • the relations panels still render (display-only)
//  • no "Add partner" / "Add parent" affordances render
// ---------------------------------------------------------------------------

test('user-role member sees read-only PersonDetail with relations panels but no edit affordances', async ({
    browser,
}) => {
    const stamp = Date.now()
    const ownerEmail = `pd-owner2-${stamp}@example.com`
    const userEmail = `pd-user-${stamp}@example.com`

    // 1) Owner context: sign in, create family, seed Anna so we have
    //    someone to view, then invite the user-role guest via REST and
    //    capture the accept link off mailpit.
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()

    await signIn(owner, ownerEmail)
    await createFamily(owner, `PDUser-${stamp}`)
    await owner.goto('/tree')
    const annaId = await addPerson(owner, 'Anna', 'Müller', '1968-08-11')
    expect(annaId).toMatch(TREE_NODE_UUID_RE)
    await closeDrawer(owner)

    const families = await owner.evaluate(async () => {
        const r = await fetch('/api/v1/families/me', { credentials: 'include' })
        const body = (await r.json()) as { data: { families: Array<{ id: string }> } }
        return body.data.families
    })
    const familyId = families[0]?.id
    if (familyId === undefined) throw new Error('owner has no families after create')

    await clearMailpit()
    await owner.evaluate(
        async ({ id, target }) => {
            await fetch(`/api/v1/families/${id}/invites`, {
                method: 'POST',
                credentials: 'include',
                headers: { 'content-type': 'application/json', 'x-family-id': id },
                body: JSON.stringify({ email: target, role: 'user' }),
            })
        },
        { id: familyId, target: userEmail },
    )

    // Pull the invite link out of mailpit before the user signs in (the
    // user's sign-in clears and replaces the latest message).
    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-fam-tree|Einladung zur Familie/.test(s), {
        recipient: userEmail,
    })
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) throw new Error('invite link not in email body')
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) throw new Error('invite link match was empty')

    // 2) User context: sign in in a separate browser context.
    const userCtx = await browser.newContext()
    const user = await userCtx.newPage()
    await signIn(user, userEmail)

    // 3) Follow the invite link to join the owner's family as `user` role.
    await user.goto(rewriteEmailLink(inviteLink))
    await expect(user).toHaveURL(/\/tree$/)

    // 4) Open Anna's drawer.
    await clickTreeNode(user, annaId)
    await expect(user.getByTestId('person-detail')).toBeVisible()

    // 5) Read-only badge is present; edit/delete buttons are NOT, neither
    //    are the "Add parent" / "Add partner" affordances.
    await expect(user.getByTestId('person-readonly-badge')).toBeVisible()
    await expect(user.getByTestId('person-edit-button')).toHaveCount(0)
    await expect(user.getByTestId('person-delete-button')).toHaveCount(0)
    await expect(user.getByTestId('person-add-parent')).toHaveCount(0)
    await expect(user.getByTestId('person-add-partner')).toHaveCount(0)

    // 6) The three relations panels still render (display-only).
    await expect(user.getByTestId('relations-parents')).toBeVisible()
    await expect(user.getByTestId('relations-partners')).toBeVisible()
    await expect(user.getByTestId('relations-children')).toBeVisible()

    await ownerCtx.close()
    await userCtx.close()
})
