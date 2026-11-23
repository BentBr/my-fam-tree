import { expect, test } from '../fixtures/console.fixture'
import { seedParentLink, seedPartnership, seedPerson } from '../page-objects/seed'
import { createFamily, signIn } from '../page-objects/session'
import { uniqueStamp } from '../page-objects/unique'

test('compare mode names the relation and lights exactly the connecting path', async ({ page }) => {
    const stamp = uniqueStamp()
    await signIn(page, `kinship-${stamp}@example.com`)
    await createFamily(page, `Kinship-${stamp}`)
    const familyId = await page.evaluate(() => localStorage.getItem('my-fam-tree:activeFamily') ?? '')
    expect(familyId).not.toBe('')

    // Grandma -> (Dad, Aunt); Dad -> Kid. Aunt and Uncle: divorced once, married again.
    const person = (given_name: string, gender: string) =>
        seedPerson(page.request, familyId, { given_name, family_name: 'Kin', gender })
    const grandma = await person('Grandma', 'female')
    const dad = await person('Dad', 'male')
    const aunt = await person('Aunt', 'female')
    const kid = await person('Kid', 'male')
    const uncle = await person('Uncle', 'male')
    const stranger = await person('Stranger', '')
    await seedParentLink(page.request, familyId, dad, grandma)
    await seedParentLink(page.request, familyId, aunt, grandma)
    await seedParentLink(page.request, familyId, kid, dad)
    await seedPartnership(page.request, familyId, aunt, uncle, {
        kind: 'marriage',
        ended_on: '2010-01-01',
        end_reason: 'divorce',
    })
    await seedPartnership(page.request, familyId, aunt, uncle, { kind: 'marriage' })

    await page.goto('/tree')
    await expect(page.getByTestId('tree-canvas')).toBeVisible()

    const litParentEdges = page.locator('[data-testid="tree-edge-parent"].highlighted')
    const litPartnerEdges = page.locator('[data-testid="tree-edge-partner"].highlighted')

    // Kid vs Aunt = aunt (2,1): three parent links, no partnership.
    await page.getByTestId('tree-compare-mode').click()
    await page.getByTestId(`tree-node-${kid}`).click()
    await page.getByTestId(`tree-node-${aunt}`).click()

    await expect(page).toHaveURL(new RegExp(`compare=${kid}(,|%2C)${aunt}`))
    await expect(page.getByTestId('kinship-banner')).toBeVisible()
    // The label follows the account locale: "aunt" (en) / "Tante" (de).
    await expect(page.getByTestId('kinship-sentence')).toContainText(/aunt|Tante/, { timeout: 10_000 })
    await expect(litParentEdges).toHaveCount(3)
    await expect(litPartnerEdges).toHaveCount(0)
    await expect(page.getByTestId(`tree-node-${stranger}`)).toHaveClass(/dimmed/)
    await expect(page.getByTestId(`tree-node-${grandma}`)).not.toHaveClass(/dimmed/)
    await expect(page.getByTestId(`tree-node-${kid}`).getByTestId('tree-node-compare-1')).toBeVisible()
    await expect(page.getByTestId(`tree-node-${aunt}`).getByTestId('tree-node-compare-2')).toBeVisible()

    // Third click replaces B. Kid vs Uncle = uncle by marriage: the same three
    // parent links plus the CURRENT marriage — the divorced row stays dark.
    await page.getByTestId(`tree-node-${uncle}`).click()
    await expect(page.getByTestId('kinship-sentence')).toContainText(/uncle|Onkel/, { timeout: 10_000 })
    await expect(litParentEdges).toHaveCount(3)
    await expect(litPartnerEdges).toHaveCount(1)
    await expect(page.locator('[data-testid="tree-edge-partner"].highlighted.ended')).toHaveCount(0)
})
