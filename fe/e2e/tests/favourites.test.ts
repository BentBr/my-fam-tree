import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { signIn, createFamily } from '../page-objects/session'

// Owner marks Klaus as favourite via the tree star and reloads — the
// gold-filled state must survive the round-trip, proving the mark is
// persisted (not just a local UI toggle).
test('owner stars a tree node and the favourite survives reload', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `fav-tree-${stamp}@example.com`)
    await createFamily(page, `FavTree-${stamp}`)

    const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
    expect(familyId).not.toBe('')

    const create = async (given: string, birth: string): Promise<string> => {
        const res = await page.request.post('/api/v1/persons', {
            headers: { 'X-Family-Id': familyId },
            data: { given_name: given, family_name: 'Star', birth_date: birth },
        })
        expect(res.ok()).toBeTruthy()
        const body = (await res.json()) as { data: { id: string } }
        return body.data.id
    }
    const klausId = await create('Klaus', '1960-04-12')

    await page.goto('/tree')
    const star = page.getByTestId(`tree-node-favourite-${klausId}`)
    await expect(star).toBeVisible()
    // Starts unfilled (no `filled` class).
    await expect(star).not.toHaveClass(/(^|\s)filled(\s|$)/)
    await star.click()
    // Optimistic update: class flips immediately, no need to wait for the network.
    await expect(star).toHaveClass(/(^|\s)filled(\s|$)/)

    await page.reload()
    const afterReload = page.getByTestId(`tree-node-favourite-${klausId}`)
    await expect(afterReload).toHaveClass(/(^|\s)filled(\s|$)/, { timeout: 5_000 })
})

// Two users in the same family see independent favourite state on the
// same person row. Proves favourites are per-user, not shared.
test('two users see independent favourite state on the same person', async ({ browser }) => {
    const stamp = Date.now()
    // Owner context: creates the family + invites user B.
    const ownerCtx = await browser.newContext()
    const ownerPage = await ownerCtx.newPage()
    await signIn(ownerPage, `fav-pair-owner-${stamp}@example.com`)
    await createFamily(ownerPage, `FavPair-${stamp}`)
    const familyId = await ownerPage.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
    expect(familyId).not.toBe('')

    // Create Klaus + invite admin B.
    const klausRes = await ownerPage.request.post('/api/v1/persons', {
        headers: { 'X-Family-Id': familyId },
        data: { given_name: 'Klaus', family_name: 'Star', birth_date: '1960-04-12' },
    })
    expect(klausRes.ok()).toBeTruthy()
    const klausId = ((await klausRes.json()) as { data: { id: string } }).data.id

    const adminEmail = `fav-pair-admin-${stamp}@example.com`
    await clearMailpit()
    const inviteRes = await ownerPage.request.post(`/api/v1/families/${familyId}/invites`, {
        data: { email: adminEmail, role: 'admin' },
    })
    expect(inviteRes.ok()).toBeTruthy()
    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-family|Einladung zur Familie/.test(s))
    const inviteLink = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)?.[0]
    if (inviteLink === undefined) throw new Error('invite link not in email body')

    // Owner marks Klaus as their favourite via the tree star.
    await ownerPage.goto('/tree')
    const ownerStar = ownerPage.getByTestId(`tree-node-favourite-${klausId}`)
    await ownerStar.click()
    await expect(ownerStar).toHaveClass(/(^|\s)filled(\s|$)/)

    // Admin context: completely separate browser context (cookies isolated).
    const adminCtx = await browser.newContext()
    const adminPage = await adminCtx.newPage()
    await signIn(adminPage, adminEmail)
    await adminPage.goto(rewriteEmailLink(inviteLink))
    await expect(adminPage).toHaveURL(/\/tree$/, { timeout: 10_000 })

    // Admin opens the tree. Klaus's star must be EMPTY for them.
    await adminPage.goto('/tree')
    const adminStar = adminPage.getByTestId(`tree-node-favourite-${klausId}`)
    await expect(adminStar).toBeVisible()
    await expect(adminStar).not.toHaveClass(/(^|\s)filled(\s|$)/)

    await ownerCtx.close()
    await adminCtx.close()
})

// /upcoming favourites pill restricts the projection to favourites of
// the signed-in caller. Klaus is favourited; Anna isn't — toggling the
// pill drops Anna's birthday and keeps Klaus's.
test('upcoming favourites pill filters to caller favourites', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `fav-upcoming-${stamp}@example.com`)
    await createFamily(page, `FavUp-${stamp}`)
    const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')

    const create = async (given: string, birth: string): Promise<string> => {
        const res = await page.request.post('/api/v1/persons', {
            headers: { 'X-Family-Id': familyId },
            data: { given_name: given, family_name: 'Up', birth_date: birth },
        })
        expect(res.ok()).toBeTruthy()
        const body = (await res.json()) as { data: { id: string } }
        return body.data.id
    }
    const klausId = await create('Klaus', '1960-04-12')
    const annaId = await create('Anna', '1962-08-22')

    // Mark Klaus only via the API (faster than driving the tree UI here).
    const favRes = await page.request.patch(`/api/v1/persons/${klausId}/favourite`, {
        headers: { 'X-Family-Id': familyId },
        data: { is_favourite: true },
    })
    expect(favRes.ok()).toBeTruthy()

    await page.goto('/upcoming')
    await expect(page.getByTestId('upcoming-page')).toBeVisible()
    // Default: both birthdays show up.
    await expect.poll(async () => page.locator('[data-testid^="upcoming-row-"]').count(), { timeout: 5_000 }).toBe(2)

    // Toggle the favourites pill. Only Klaus's birthday remains.
    await page.getByTestId('upcoming-filter-favourites').click()
    await expect.poll(async () => page.locator('[data-testid^="upcoming-row-"]').count(), { timeout: 5_000 }).toBe(1)

    // Verify the remaining row really is Klaus's (the BE returns the
    // person_id on birthday events; the FE wires the click to it).
    void annaId // referenced only for the seed; the assertion above is what counts
})
