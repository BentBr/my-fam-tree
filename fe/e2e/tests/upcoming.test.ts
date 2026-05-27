import { expect, test } from '../fixtures/console.fixture'
import { signIn, createFamily } from '../page-objects/session'

test('upcoming dates page filters by birthday + anniversary toggles', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `upcoming-${stamp}@example.com`)
    await createFamily(page, `Upcoming-${stamp}`)

    const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
    expect(familyId).not.toBe('')

    // Seed a small family graph via the API directly: two people with
    // different birthdays + one open partnership (wedding anniversary).
    const create = async (given: string, family: string, birth: string): Promise<string> => {
        const res = await page.request.post('/api/v1/persons', {
            headers: { 'X-Family-Id': familyId },
            data: { given_name: given, family_name: family, birth_date: birth },
        })
        expect(res.ok()).toBeTruthy()
        const body = (await res.json()) as { data: { id: string } }
        return body.data.id
    }
    const a = await create('Anna', 'Schmidt', '1985-06-04')
    const b = await create('Ben', 'Schmidt', '1986-09-22')
    // Open marriage ⇒ wedding_anniversary event.
    const partRes = await page.request.post('/api/v1/partnerships', {
        headers: { 'X-Family-Id': familyId },
        data: { partner_a_id: a, partner_b_id: b, kind: 'marriage', started_on: '2010-07-11' },
    })
    expect(partRes.ok()).toBeTruthy()

    // Navigate via the sidebar nav so the route + drawer wiring is exercised.
    await page.goto('/upcoming')
    await expect(page.getByTestId('upcoming-page')).toBeVisible()

    // 2 birthdays + 1 wedding = 3 rows by default (filter=all). Poll
    // because the useUpcoming query is in flight when page.goto resolves
    // — the page shell appears first, the rows trail by one fetch tick.
    await expect
        .poll(async () => page.locator('[data-testid^="upcoming-row-"]').count(), {
            timeout: 5_000,
        })
        .toBe(3)

    // Click "Birthday" — only birthdays should remain (2).
    await page.getByTestId('upcoming-filter-birthday').click()
    await expect.poll(async () => page.locator('[data-testid^="upcoming-row-"]').count(), { timeout: 5_000 }).toBe(2)
    const allRowsAfterBirthday = await page.locator('[data-testid^="upcoming-row-"]').all()
    for (const row of allRowsAfterBirthday) {
        const tid = await row.getAttribute('data-testid')
        expect(tid).toBe('upcoming-row-birthday')
    }

    // Click "Birthday" again — filter reverts to "all" (3 rows again).
    await page.getByTestId('upcoming-filter-birthday').click()
    await expect.poll(async () => page.locator('[data-testid^="upcoming-row-"]').count(), { timeout: 5_000 }).toBe(3)

    // Click "Jahrestag" — only the wedding anniversary should remain.
    await page.getByTestId('upcoming-filter-anniversary').click()
    await expect.poll(async () => page.locator('[data-testid^="upcoming-row-"]').count(), { timeout: 5_000 }).toBe(1)
    await expect(page.locator('[data-testid="upcoming-row-wedding_anniversary"]')).toHaveCount(1)
})
