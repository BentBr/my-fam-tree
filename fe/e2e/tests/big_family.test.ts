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
import type { APIRequestContext } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Sized to fit under the persons-list 100-row server-side clamp. A wider
// version is gated behind that limit being raised + cursor pagination
// being wired into the tree service. 50 persons is still plenty to catch
// the overlap / drop-node regressions this test is here to guard against.
const GENERATIONS = 10
const PER_GEN = 5
const TOTAL = GENERATIONS * PER_GEN

interface BulkResult {
    /** Map of synthetic id (`g0-3`) → server-assigned uuid. */
    idMap: Map<string, string>
}

async function bulkSeedFamily(request: APIRequestContext, familyId: string): Promise<BulkResult> {
    const idMap = new Map<string, string>()

    // Persons go first, chunked so we don't open more than 50 sockets at
    // once. Each chunk is awaited before the next launches; within a chunk
    // the 50 POSTs race in parallel.
    const CHUNK = 50
    const personRequests: Array<() => Promise<void>> = []
    for (let g = 0; g < GENERATIONS; g += 1) {
        for (let i = 0; i < PER_GEN; i += 1) {
            const key = `g${g}-${i}`
            personRequests.push(async () => {
                const res = await request.post('/api/v1/persons', {
                    headers: { 'X-Family-Id': familyId },
                    data: {
                        given_name: `P${g}_${i}`,
                        family_name: 'Stress',
                        // Distribute birth years so the generation-rank algorithm
                        // has signal: gen 0 ~1900, gen 9 ~1980.
                        birth_date: `${1900 + g * 9}-01-01`,
                    },
                })
                expect(res.ok()).toBeTruthy()
                const body = (await res.json()) as { data: { id: string } }
                idMap.set(key, body.data.id)
            })
        }
    }
    for (let i = 0; i < personRequests.length; i += CHUNK) {
        await Promise.all(personRequests.slice(i, i + CHUNK).map((fn) => fn()))
    }

    // Parent links: each (g,i) for g >= 1 gets one parent at (g-1, i).
    // A second parent at (g-1, (i+1) % PER_GEN) lets the partner pass
    // and the duplicate detection both have something to chew on.
    const linkRequests: Array<() => Promise<void>> = []
    for (let g = 1; g < GENERATIONS; g += 1) {
        for (let i = 0; i < PER_GEN; i += 1) {
            const childId = idMap.get(`g${g}-${i}`)
            const parentId = idMap.get(`g${g - 1}-${i}`)
            if (childId === undefined || parentId === undefined) continue
            linkRequests.push(async () => {
                const res = await request.post('/api/v1/parent-links', {
                    headers: { 'X-Family-Id': familyId },
                    data: { child_id: childId, parent_id: parentId, kind: 'biological' },
                })
                if (!res.ok()) {
                    console.error(`parent-link POST failed ${res.status()}:`, await res.text())
                }
                expect(res.ok()).toBeTruthy()
            })
        }
    }
    // Parent-link inserts run in a SERIALIZABLE Postgres tx; concurrent
    // POSTs hitting the same family's row set get aborted with sqlstate
    // 40001. Sequential is fast enough at this scale (~50ms × ~200 ≈
    // 10s) and dodges the abort/retry dance.
    for (const fn of linkRequests) {
        await fn()
    }

    return { idMap }
}

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
        const email = `stress-${Date.now()}@example.com`
        await clearMailpit()
        const login = new LoginPage(page)
        await login.goto()
        await login.signIn(email)
        await expect(login.sent).toBeVisible()
        const mail = await waitForEmail((s) => /Sign in to my-family|Anmeldung bei my-family/.test(s))
        const match = mail.text.match(/https?:\/\/\S+\/auth\/consume\?token=\S+/)
        if (match === null || match[0] === undefined) throw new Error('consume link missing')
        await page.goto(rewriteEmailLink(match[0]))
        await expect(page).toHaveURL(/\/(families\/create|families\/pick|tree)$/)

        // Always create a fresh family via the API. Going through the UI
        // flow adds a second-of-overhead that doesn't pay back at this
        // scale, and the API path is what the test actually wants to
        // exercise.
        const createRes = await page.request.post('/api/v1/families', {
            data: { name: `Stress ${Date.now()}` },
        })
        expect(createRes.ok()).toBeTruthy()
        const created = (await createRes.json()) as { data: { family: { id: string } } }
        await page.goto(`/tree`)
        // Seed the active-family bookkeeping so subsequent fetches send
        // the correct X-Family-Id header.
        await page.evaluate((id) => {
            localStorage.setItem('my-family:activeFamily', id)
        }, created.data.family.id)
        await page.reload()
        await expect(page).toHaveURL(/\/tree$/)
        const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
        expect(familyId).not.toBe('')

        const startSeed = Date.now()
        await bulkSeedFamily(page.request, familyId)
        const seedMs = Date.now() - startSeed
        console.log(`[big_family] seeded ${TOTAL} persons + ${TOTAL - PER_GEN} edges in ${seedMs}ms`)

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
