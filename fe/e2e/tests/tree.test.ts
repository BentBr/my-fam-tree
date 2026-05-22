import type { Page } from '@playwright/test'
import { expect, test } from '../fixtures/console.fixture'

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
    await expect(page).toHaveURL(/\/(tree|health|families\/create|families\/pick)$/)
}

async function createFamily(page: Page, name: string): Promise<void> {
    await page.goto('/families/create')
    await page.getByTestId('family-name').locator('input').fill(name)
    await page.getByTestId('family-create-submit').click()
    await expect(page).toHaveURL(/\/tree$/)
}

// The TreeNode SVG emits sibling `<text>` elements with non-UUID testids
// (`tree-node-name`, `tree-node-birth`, `tree-node-death`) for granular
// assertions, so a bare `[data-testid^="tree-node-"]` selector returns
// 2–4 hits per actual person. We pull all matches and keep only the
// UUID-shaped ids — the outer `<g data-testid="tree-node-<uuid>">` groups.
const TREE_NODE_UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

async function listTreeNodeIds(page: Page): Promise<string[]> {
    const raw = await page
        .locator('[data-testid^="tree-node-"]')
        .evaluateAll((els) => els.map((el) => (el.getAttribute('data-testid') ?? '').replace('tree-node-', '')))
    return raw.filter((id) => TREE_NODE_UUID_RE.test(id))
}

/**
 * Adds a person via the create drawer.  Returns the new person's id, read off
 * the `data-testid="tree-node-<uuid>"` attribute that lands once the drawer
 * has switched into the post-save detail view.
 */
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

    // After save the drawer flips to the detail view for the new person; poll
    // until the post-mutation tree refetch settles and exactly one new UUID
    // appears in the canvas (the SVG re-render lags the mutation by a tick).
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

test('empty tree shows the CTA card and opens the create drawer on click', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `empty-tree-${stamp}@example.com`)
    await createFamily(page, `Empty-${stamp}`)

    await page.goto('/tree')

    // No persons yet: the empty-state card should render in place of the SVG.
    await expect(page.getByTestId('tree-empty')).toBeVisible()
    await expect(page.getByTestId('tree-canvas')).toHaveCount(0)

    // Click the CTA button (not the outer card) — exercises the same path as
    // the toolbar "Add person" button.
    await page.getByTestId('tree-empty-cta').click()
    await expect(page.getByTestId('person-edit')).toBeVisible()

    // Sanity-check: the surrounding outer-card click also works. Close the
    // drawer first, then click the card's title to reopen.
    await page.getByTestId('person-edit-cancel').click()
    await expect(page.locator('.v-navigation-drawer__scrim')).toHaveCount(0)
    await page.getByTestId('tree-empty').click()
    await expect(page.getByTestId('person-edit')).toBeVisible()
})

test('family switcher "create new" routes to /families/create', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `fam-switch-${stamp}@example.com`)
    await createFamily(page, `Switch-${stamp}`)

    // After createFamily the redirect lands on /health; switcher is in AppBar
    // and is reachable from any authenticated route.
    await page.goto('/tree')
    await expect(page.getByTestId('family-switcher')).toBeVisible()

    // Open the v-select overlay and pick the "Create new family…" entry by
    // its visible text. Locale auto-resolves the German label too.
    await page.getByTestId('family-switcher').click()
    await page.getByRole('option', { name: /Create new family|Neue Familie anlegen/ }).click()

    await expect(page).toHaveURL(/\/families\/create$/)
    await expect(page.getByTestId('family-name')).toBeVisible()
})

test('owner adds people, links a parent and a partner, tree renders edges', async ({ page }) => {
    // Unique per-run email + family so the truncate-on-teardown can't race a
    // re-run before postgres has finished cleanup (defense in depth — global
    // teardown already truncates, but timestamped data keeps the test idempotent).
    const stamp = Date.now()
    await signIn(page, `tree-${stamp}@example.com`)
    await createFamily(page, `Tree-${stamp}`)

    await page.goto('/tree')
    // Fresh family — `tree-empty` is rendered in place of `tree-canvas` until
    // the first person is added. Add Anna; the canvas takes over from there.
    await expect(page.getByTestId('tree-empty')).toBeVisible()

    // 1. Add Anna.
    const annaId = await addPerson(page, 'Anna', 'Müller', '1980-04-15')
    await expect(page.getByTestId('tree-canvas')).toBeVisible()
    expect(annaId).not.toBe('')
    await closeDrawer(page)

    // 2. Add Otto and link him as Anna's parent.
    const ottoId = await addPerson(page, 'Otto', 'Müller', '1950-01-01')
    expect(ottoId).toBe(ottoId) // tautology; keeps lint happy without dropping the binding
    await closeDrawer(page)

    await clickTreeNode(page, annaId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    // The "Add parent" v-select lives inside the Parents v-expansion-panel
    // which is rendered collapsed by default. Click the panel title to
    // expand before the v-select trigger becomes pointer-clickable.
    await page.getByTestId('relations-parents').click()
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
    // Same expansion-panel gating for the Partners section.
    await page.getByTestId('relations-partners').click()
    await page.getByTestId('person-add-partner').click()
    await page.getByRole('option', { name: 'Maria Schmidt' }).click()
    // Partner-kind has no default — must pick one before submit enables.
    await page.getByTestId('person-add-partner-kind').click()
    await page.getByRole('option', { name: /Marriage|Ehe/ }).click()
    await page.getByTestId('person-add-partner-submit').click()

    // At least one partner edge appears on the canvas.
    await expect(page.locator('[data-testid="tree-edge-partner"]').first()).toBeVisible()
    const partnerCount = await page.locator('[data-testid="tree-edge-partner"]').count()
    expect(partnerCount).toBeGreaterThanOrEqual(1)
})

// Hover focus pass: when the user hovers Klaus the canvas should mark his
// direct relations (parents Otto + Hannelore, partner Anna, children Lina +
// Max) with the `related` class, while non-related members (Werner, Greta,
// peter old) get `dimmed`. Builds the seeded family graph inline so the
// test doesn't depend on the seeder having run.
test('hovering Klaus highlights direct relations and dims the rest', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `hover-${stamp}@example.com`)
    await createFamily(page, `Hover-${stamp}`)
    await page.goto('/tree')

    const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
    expect(familyId).not.toBe('')

    // Helper: create a person + return its server-assigned UUID. Uses the
    // page.request context so the cookie jar + X-Family-Id header reach
    // the API without any UI clicks.
    const create = async (given: string, family: string, birth: string): Promise<string> => {
        const res = await page.request.post('/api/v1/persons', {
            headers: { 'X-Family-Id': familyId },
            data: { given_name: given, family_name: family, birth_date: birth },
        })
        expect(res.ok()).toBeTruthy()
        const body = (await res.json()) as { data: { id: string } }
        return body.data.id
    }

    // Build the seeded family graph (Müller + Schmidt) plus peter old as
    // Otto's parent, so the test mirrors the visual layout the user filed
    // the report against. Brigitte + Felix add the half-sibling case (Felix
    // is Klaus + Brigitte's biological son, half-brother to Lina + Max).
    const otto = await create('Otto', 'Müller', '1935-03-12')
    const hannelore = await create('Hannelore', 'Müller', '1938-07-23')
    const werner = await create('Werner', 'Schmidt', '1936-05-18')
    const greta = await create('Greta', 'Schmidt', '1940-02-09')
    const klaus = await create('Klaus', 'Müller', '1965-04-22')
    const anna = await create('Anna', 'Müller', '1968-08-11')
    const brigitte = await create('Brigitte', 'Mayer', '1968-11-30')
    const felix = await create('Felix', 'Müller', '1992-07-08')
    const lina = await create('Lina', 'Müller', '1995-12-03')
    const max = await create('Max', 'Müller', '1998-04-17')
    const emma = await create('Emma', 'Müller', '2020-05-10')
    const peter = await create('peter', 'old', '1910-05-20')

    const link = async (childId: string, parentId: string): Promise<void> => {
        const res = await page.request.post('/api/v1/parent-links', {
            headers: { 'X-Family-Id': familyId },
            data: { child_id: childId, parent_id: parentId, kind: 'biological' },
        })
        expect(res.ok()).toBeTruthy()
    }
    interface PartnershipBody {
        partner_a_id: string
        partner_b_id: string
        kind: string
        ended_on?: string
        end_reason?: string
    }
    const partner = async (
        aId: string,
        bId: string,
        opts?: { ended_on?: string; end_reason?: string },
    ): Promise<void> => {
        const body: PartnershipBody = { partner_a_id: aId, partner_b_id: bId, kind: 'marriage' }
        if (opts?.ended_on !== undefined) body.ended_on = opts.ended_on
        if (opts?.end_reason !== undefined) body.end_reason = opts.end_reason
        const res = await page.request.post('/api/v1/partnerships', {
            headers: { 'X-Family-Id': familyId },
            // Field names match `PartnershipCreateReq` in
            // `crates/api/src/routes/partnerships.rs` — `partner_a_id` /
            // `partner_b_id`, not `a_id` / `b_id`. The shorter form was a
            // copy-paste from the relationships tree edge JSON shape.
            data: body,
        })
        if (!res.ok()) {
            console.error(`partnership POST failed ${res.status()}:`, await res.text())
        }
        expect(res.ok()).toBeTruthy()
    }

    // Parent links: Otto→peter, Klaus→Otto+Hannelore, Anna→Werner+Greta,
    // Felix→Klaus+Brigitte, Lina+Max→Klaus+Anna, Emma→Lina.
    await link(otto, peter)
    await link(klaus, otto)
    await link(klaus, hannelore)
    await link(anna, werner)
    await link(anna, greta)
    await link(felix, klaus)
    await link(felix, brigitte)
    await link(lina, klaus)
    await link(lina, anna)
    await link(max, klaus)
    await link(max, anna)
    await link(emma, lina)
    // Partnerships: Otto+Hannelore, Werner+Greta, Klaus+Anna, Klaus+Brigitte
    // (divorced 2000 so the ex-spouse-adjacency layout pass threads
    // Brigitte to Klaus's LEFT, current partner Anna to his RIGHT).
    await partner(otto, hannelore)
    await partner(werner, greta)
    await partner(klaus, anna)
    await partner(klaus, brigitte, { ended_on: '2000-06-30', end_reason: 'divorce' })

    await page.reload()
    await expect(page.getByTestId('tree-canvas')).toBeVisible()
    await expect(page.getByTestId(`tree-node-${klaus}`)).toBeVisible()

    // Hover Klaus directly via the SVG node's `mouseenter` (dispatchEvent
    // bypasses the viewport/visibility check — the canvas pans/zooms so
    // the absolute position may be outside the CSS viewport rect that
    // Playwright would otherwise validate against).
    await page.getByTestId(`tree-node-${klaus}`).dispatchEvent('mouseenter')

    const klassesOf = async (id: string): Promise<string[]> =>
        page.getByTestId(`tree-node-${id}`).evaluate((el) => Array.from((el as Element).classList))

    // Klaus itself carries the `hovered` class; his direct relations carry
    // `related`. We poll briefly so the Vue reactivity tick lands.
    await expect.poll(async () => (await klassesOf(klaus)).includes('hovered')).toBe(true)
    for (const id of [otto, hannelore, anna, lina, max]) {
        await expect.poll(async () => (await klassesOf(id)).includes('related')).toBe(true)
    }
    // Werner / Greta / peter old are NOT direct relations — they should
    // land with `dimmed` instead of `related`.
    for (const id of [werner, greta, peter]) {
        const cls = await klassesOf(id)
        expect(cls).not.toContain('related')
        expect(cls).toContain('dimmed')
    }

    // Tree layout v3 invariants. Each TreeNode is wrapped in a `<g
    // transform="translate(x, y)">`, so we read the y component out of
    // the transform attribute and compare. All of Klaus's children
    // (Felix, Lina, Max) must sit at the same y even though only Lina
    // has a descendant (Emma) — pre-v3 bottom-up depth dropped Felix +
    // Max one row below Lina (Bug 1 in the user's report).
    const yOf = async (id: string): Promise<number> =>
        page.getByTestId(`tree-node-${id}`).evaluate((el) => {
            const transform = el.getAttribute('transform') ?? ''
            const m = transform.match(/translate\(\s*[^,]+,\s*([-\d.]+)\s*\)/)
            return m !== null && m[1] !== undefined ? Number.parseFloat(m[1]) : Number.NaN
        })
    const yFelix = await yOf(felix)
    const yLina = await yOf(lina)
    const yMax = await yOf(max)
    expect(yFelix).toBe(yLina)
    expect(yFelix).toBe(yMax)
    // Klaus's row sits strictly above Felix/Lina/Max's row.
    const yKlaus = await yOf(klaus)
    expect(yKlaus).toBeLessThan(yFelix)
    // Brigitte (a root partner of Klaus) shares Klaus's row — Bug 2.
    const yBrigitte = await yOf(brigitte)
    expect(yBrigitte).toBe(yKlaus)

    // v3.1 ex-spouse adjacency: Brigitte is Klaus's divorced (ended_on
    // 2000) first wife; Anna is the current partner. The multi-couple
    // block threads Brigitte to Klaus's LEFT and Anna to his RIGHT.
    const xOf = async (id: string): Promise<number> =>
        page.getByTestId(`tree-node-${id}`).evaluate((el) => {
            const transform = el.getAttribute('transform') ?? ''
            const m = transform.match(/translate\(\s*([-\d.]+)/)
            return m !== null && m[1] !== undefined ? Number.parseFloat(m[1]) : Number.NaN
        })
    const xBrigitte = await xOf(brigitte)
    const xKlaus = await xOf(klaus)
    const xAnna = await xOf(anna)
    expect(xBrigitte).toBeLessThan(xKlaus)
    expect(xKlaus).toBeLessThan(xAnna)
    // peter old (1910) sits strictly above Otto's row.
    const yOtto = await yOf(otto)
    const yPeter = await yOf(peter)
    expect(yPeter).toBeLessThan(yOtto)

    // Zoom defaults: the SVG inner `<g>` carries the d3-zoom transform.
    // The mount path either focuses on `currentUserId` at FOCUS_SCALE
    // (0.75) or clamps fit-to-view at MIN_FIT_SCALE (0.5). Either way
    // the resulting scale should be ≥ MIN_FIT_SCALE for the seeded 12-
    // person family — Bug 3 / 4 in the user's report. We read the
    // transform off the root `<g>` and parse the scale component.
    const initialScale = await page.locator('[data-testid="tree-canvas"] svg > g').evaluate((el) => {
        const t = el.getAttribute('transform') ?? ''
        const m = t.match(/scale\(([-\d.]+)/)
        return m !== null && m[1] !== undefined ? Number.parseFloat(m[1]) : Number.NaN
    })
    expect(initialScale).toBeGreaterThanOrEqual(0.5)
})
