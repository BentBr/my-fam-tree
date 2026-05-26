import type { Page } from '@playwright/test'
import { expect, test } from '../fixtures/console.fixture'

import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Standard sign-in dance — same shape as upcoming.test.ts / tree.test.ts.
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

async function createFamily(page: Page, name: string): Promise<void> {
    await page.goto('/families/create')
    await page.getByTestId('family-name').locator('input').fill(name)
    await page.getByTestId('family-create-submit').click()
    await expect(page).toHaveURL(/\/tree$/)
}

// Returns an ISO date `daysFromNow` calendar days in the future but with
// the year set 30 years in the past, so Upcoming reads it as a 30th
// birthday (not "tomorrow's birth"). Keeps the row ordering predictable.
function birthdayInDays(daysFromNow: number): string {
    const target = new Date()
    target.setDate(target.getDate() + daysFromNow)
    target.setFullYear(target.getFullYear() - 30)
    const yyyy = target.getFullYear()
    const mm = String(target.getMonth() + 1).padStart(2, '0')
    const dd = String(target.getDate()).padStart(2, '0')
    return `${yyyy}-${mm}-${dd}`
}

test('upcoming row click centers the person and opens the drawer', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `tree-deep-${stamp}@example.com`)
    await createFamily(page, `DeepLink-${stamp}`)

    const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
    expect(familyId).not.toBe('')

    // One person with a near-future birthday so Upcoming has exactly one row.
    const createRes = await page.request.post('/api/v1/persons', {
        headers: { 'X-Family-Id': familyId },
        data: {
            given_name: 'Centerable',
            family_name: 'Person',
            birth_date: birthdayInDays(3),
        },
    })
    expect(createRes.ok()).toBeTruthy()
    const created = (await createRes.json()) as { data: { id: string } }
    const personId = created.data.id

    await page.goto('/upcoming')
    await expect(page.getByTestId('upcoming-page')).toBeVisible()
    const row = page.getByTestId('upcoming-row-birthday').first()
    await expect(row).toBeVisible()
    await row.click()

    // 1. URL ends at /tree (the `?center=` was stripped).
    await expect(page).toHaveURL(/\/tree$/)

    // 2. The right-hand drawer shows the clicked person. The drawer
    //    transition + the GET person query both have to settle, so wait
    //    on the inner section first; the title only renders once the
    //    query resolves.
    await expect(page.getByTestId('person-drawer')).toBeVisible()
    await expect(page.getByTestId('person-detail')).toBeVisible({ timeout: 10_000 })
    await expect(page.getByTestId('person-detail-title')).toHaveText(/Centerable Person/, { timeout: 10_000 })

    // 3. The corresponding TreeNode is rendered. Presence is the
    //    strongest assertion we can make without leaking transform-matrix
    //    internals into the test; the data-testid embeds the person id.
    await expect(page.locator(`[data-testid="tree-node-${personId}"]`)).toBeVisible()
})

test('second upcoming click reopens the drawer (cached tree.data path)', async ({ page }) => {
    // Regression: after the first /tree visit the tree query is hot
    // in cache. A naive "open drawer on tree.data immediate" watcher
    // fires synchronously during setup on the second visit, which
    // puts the drawer through Vuetify's "mounted already open" no-op
    // trap. The fix gates the watcher on a post-mount isMounted ref
    // so the open always happens after first paint, cached or not.
    const stamp = Date.now()
    await signIn(page, `tree-deep-multi-${stamp}@example.com`)
    await createFamily(page, `DeepLinkMulti-${stamp}`)

    const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
    expect(familyId).not.toBe('')

    const create = async (given: string, days: number): Promise<string> => {
        const res = await page.request.post('/api/v1/persons', {
            headers: { 'X-Family-Id': familyId },
            data: { given_name: given, family_name: 'Multi', birth_date: birthdayInDays(days) },
        })
        const body = (await res.json()) as { data: { id: string } }
        return body.data.id
    }
    const firstId = await create('First', 3)
    const secondId = await create('Second', 7)

    // First click — proves the fresh path works.
    await page.goto('/upcoming')
    await page.getByTestId(`upcoming-row-birthday`).first().click()
    await expect(page.getByTestId('person-detail')).toBeVisible({ timeout: 10_000 })
    await expect(page.getByTestId('person-detail-title')).toHaveText(/First Multi/, { timeout: 10_000 })

    // Back to /upcoming and click the SECOND row. Tree data is now
    // cached; without the post-mount gate the drawer stays closed.
    await page.goto('/upcoming')
    await expect(page.getByTestId('upcoming-page')).toBeVisible()
    // Click the row whose label mentions "Second" — order may vary.
    await page.locator('[data-testid="upcoming-row-birthday"]').filter({ hasText: /Second Multi/ }).first().click()

    await expect(page).toHaveURL(/\/tree$/)
    await expect(page.getByTestId('person-detail')).toBeVisible({ timeout: 10_000 })
    await expect(page.getByTestId('person-detail-title')).toHaveText(/Second Multi/, { timeout: 10_000 })
    await expect(page.locator(`[data-testid="tree-node-${secondId}"]`)).toBeVisible()

    // Avoid unused-var noise.
    expect(firstId).toBeTruthy()
})
