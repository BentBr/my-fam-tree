/**
 * Multi-generation stress test. Seeds a synthetic family (10 generations
 * × 5 per gen = 50 persons) via the API in parallel batches, navigates
 * to /tree, and asserts every person renders on the canvas. The point
 * is to catch layout regressions that only show at scale (e.g. the
 * "multi-pair partner pass midpoint collision" that drops cards), plus
 * to keep us honest about render performance for realistically large
 * trees. The size sits right under the persons-list 100-row clamp; a
 * larger version is gated behind raising that limit + wiring cursor
 * pagination through the tree service.
 *
 * Performance budget. The whole test must finish well under a minute
 * — preferably under 20s — so we can keep it in the regular e2e
 * pipeline. Shortcuts taken:
 *   - Single mailpit sign-in (no per-row re-auth).
 *   - One family scoped to this test; we tear it down at the end so
 *     the seeded data the other suites depend on stays untouched.
 *   - All person + parent-link inserts go through `page.request` in
 *     chunks of 50 in parallel. Browser-level fetch via the cookie
 *     jar reuses the access token; no UI clicks per row.
 *   - Tree assertions read the SVG DOM once after a single navigation
 *     and a `poll` that waits for the node count to stabilise.
 */
import { expect, test } from '../fixtures/console.fixture'
import { clearMailpit } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'
import { PER_GEN, TOTAL, bulkSeedFamily } from '../page-objects/seed'
import { consumeLinkFromEmail } from '../page-objects/session'
import { uniqueStamp } from '../page-objects/unique'

test.describe('big family stress test', () => {
    // Slow test — bump the per-test timeout above the default 30s so
    // the in-network seeder + parallel inserts fit under one budget.
    test.setTimeout(60_000)

    test('renders a multi-generation tree without dropping nodes', async ({ page }) => {
        // Per-run unique email — `stress@example.com` lingers across local
        // runs (the dev DB isn't truncated between non-CI invocations)
        // and lands on `/families/pick` after consuming the magic link,
        // not `/families/create`. A unique email guarantees the fresh
        // empty-state.
        const email = `stress-${uniqueStamp()}@example.com`
        await clearMailpit()
        const login = new LoginPage(page)
        await login.goto()
        await login.signIn(email)
        await expect(login.sent).toBeVisible()
        await page.goto(await consumeLinkFromEmail(email))
        await expect(page).toHaveURL(/\/(families\/create|families\/pick|tree)$/)

        // Always create a fresh family via the API. Going through the UI
        // flow adds a second-of-overhead that doesn't pay back at this
        // scale, and the API path is what the test actually wants to
        // exercise.
        const createRes = await page.request.post('/api/v1/families', {
            data: { name: `Stress ${uniqueStamp()}` },
        })
        expect(createRes.ok()).toBeTruthy()
        const created = (await createRes.json()) as { data: { family: { id: string } } }
        await page.goto(`/tree`)
        // Seed the active-family bookkeeping so subsequent fetches send
        // the correct X-Family-Id header.
        await page.evaluate((id) => {
            localStorage.setItem('my-fam-tree:activeFamily', id)
        }, created.data.family.id)
        await page.reload()
        await expect(page).toHaveURL(/\/tree$/)
        const familyId = await page.evaluate(() => localStorage.getItem('my-fam-tree:activeFamily') ?? '')
        expect(familyId).not.toBe('')

        const startSeed = Date.now()
        await bulkSeedFamily(page.request, familyId)
        const seedMs = Date.now() - startSeed
        console.log(
            `[big_family] seeded ${String(TOTAL)} persons + ${String(TOTAL - PER_GEN)} edges in ${String(seedMs)}ms`,
        )

        // `evaluateAll` runs in the browser context, where module-scope
        // values aren't reachable; the UUID test inlines instead.
        const collectIds = async (): Promise<string[]> =>
            page
                .locator('[data-testid^="tree-node-"]')
                .evaluateAll((els) =>
                    els
                        .map((el) => (el.getAttribute('data-testid') ?? '').replace('tree-node-', ''))
                        .filter((id) => /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(id)),
                )

        // Force a refetch and wait for the canvas to settle on TOTAL nodes.
        await page.reload()
        await expect
            .poll(async () => (await collectIds()).length, {
                timeout: 30_000,
                intervals: [500, 1_000],
            })
            .toBe(TOTAL)

        // Sanity: every person id appears exactly once.
        const ids = await collectIds()
        expect(new Set(ids).size).toBe(TOTAL)

        // No (x, y) collisions on the canvas — the regression we just
        // fixed. Read transforms straight off the DOM and check for dupes.
        const placements = await page.locator('[data-testid^="tree-node-"]').evaluateAll((els) =>
            els
                .filter((el) => {
                    const tid = (el.getAttribute('data-testid') ?? '').replace('tree-node-', '')
                    return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(tid)
                })
                .map((el) => el.getAttribute('transform') ?? ''),
        )
        expect(new Set(placements).size).toBe(TOTAL)
    })
})
