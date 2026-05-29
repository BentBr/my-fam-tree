// Phase 5 Task 23 — invite / family edge cases (the "wrong Müller" worry).
//
// Three end-to-end scenarios:
//   1. Invite an existing account + idempotent re-accept (token single-use,
//      second visit must surface gracefully, not 500).
//   2. Duplicate family names are allowed AND distinguishable in the picker
//      (created-date subtitle appears only when a name actually repeats).
//   3. Inviting B into "Müller #2" while B is already in "Müller #1" lands B
//      as a member of both with the token's family active — and B sees that
//      family's persons only, no cross-family leak.

import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { createFamily, signIn } from '../page-objects/session'

/**
 * Owner sends an API invite at `role` and the invitee (which may or may not
 * be already signed in) accepts via the magic link. Returns the parsed invite
 * URL so callers can re-visit it for the idempotency check.
 */
async function inviteAndAccept(
    ownerPage: Page,
    inviteePage: Page,
    familyId: string,
    inviteeEmail: string,
    role: 'user' | 'admin',
): Promise<string> {
    await clearMailpit()
    const inviteRes = await ownerPage.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: inviteeEmail, role },
    })
    expect(inviteRes.ok()).toBeTruthy()
    const mail = await waitForEmail((s) => /Join the .+ family on my-fam-tree|Einladung zur Familie/.test(s))
    const m = mail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (m === null) throw new Error('invite link not in email')
    const link = m[0]
    if (link === undefined) throw new Error('invite link empty')
    await inviteePage.goto(rewriteEmailLink(link))
    await expect(inviteePage).toHaveURL(/\/(tree|invite\/accept)/)
    return link
}

/**
 * Create a second family with a name you already own, clicking through the
 * soft-confirm dialog. The existing `createFamily(page, name)` helper assumes
 * a unique name (no dialog) — this is the duplicate-aware sibling.
 */
async function createOwnedDuplicate(page: Page, name: string): Promise<string> {
    await page.goto('/families/create')
    await page.getByTestId('family-name').locator('input').fill(name)
    await page.getByTestId('family-create-submit').click()
    // Soft-confirm dialog opens because the name matches one we already own.
    await expect(page.getByTestId('family-duplicate-dialog')).toBeVisible()
    await page.getByTestId('family-duplicate-confirm').click()
    await expect(page).toHaveURL(/\/tree$/)
    return page.evaluate(() => localStorage.getItem('my-fam-tree:activeFamily') ?? '')
}

test('23.1: existing-account invite + re-accepting the same link is graceful, not a 500', async ({ browser }) => {
    const stamp = Date.now()

    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    await signIn(owner, `edge-owner-${stamp}@example.com`)
    const familyId = await createFamily(owner, `EdgeFam-${stamp}`)

    // Guest signs in FIRST — they're an "existing account" with no families.
    const guestCtx = await browser.newContext()
    const guest = await guestCtx.newPage()
    const guestEmail = `edge-guest-${stamp}@example.com`
    await signIn(guest, guestEmail)

    // First accept — happy path.
    const inviteLink = await inviteAndAccept(owner, guest, familyId, guestEmail, 'user')
    await expect(guest.getByTestId('family-switcher')).toContainText(`EdgeFam-${stamp}`, { timeout: 15_000 })

    // Same token, again — server marks the token consumed on first accept, so the
    // second invite::accept returns MagicLinkInvalid. InviteAccept catches it
    // and surfaces an inert error card — NOT a 500, NOT an unhandled rejection.
    await guest.goto(rewriteEmailLink(inviteLink))
    await expect(guest.getByTestId('invite-error')).toBeVisible({ timeout: 15_000 })

    await guestCtx.close()
    await ownerCtx.close()
})

test('23.2: same user creates two same-named families — both appear and are distinguishable', async ({ browser }) => {
    const stamp = Date.now()
    const name = `Müller-${stamp}`

    const ctx = await browser.newContext()
    const page = await ctx.newPage()
    await signIn(page, `edge-dup-${stamp}@example.com`)

    // First family — unique name path (no dialog).
    const id1 = await createFamily(page, name)
    expect(id1).not.toBe('')

    // Second family with the SAME name — must trip the soft-confirm dialog.
    const id2 = await createOwnedDuplicate(page, name)
    expect(id2).not.toBe('')
    expect(id2).not.toBe(id1)

    // Both visible in /families/pick (exempt from the family guard) with the
    // disambiguator subtitle — "owner · created <date>" — since the name
    // actually repeats. Each row gets its own created-date.
    await page.goto('/families/pick')
    const row1 = page.locator(`[data-testid="pick-${id1}"]`)
    const row2 = page.locator(`[data-testid="pick-${id2}"]`)
    await expect(row1).toBeVisible()
    await expect(row2).toBeVisible()
    // Subtitle on both rows mentions a creation date (locale-formatted),
    // proving the disambiguator fired. The exact date string varies by clock
    // and locale; assert presence of "created" + a 4-digit year.
    await expect(row1).toContainText(/created.*\d{4}/)
    await expect(row2).toContainText(/created.*\d{4}/)

    await ctx.close()
})

test('23.3: invited into the correct same-named family — no cross-family person leak', async ({ browser }) => {
    const stamp = Date.now()
    const sameName = `Müller-${stamp}`

    // Owner A creates Müller #1 and a person "AliceA-{stamp}" in it.
    const aCtx = await browser.newContext()
    const a = await aCtx.newPage()
    await signIn(a, `edge-a-${stamp}@example.com`)
    const fam1 = await createFamily(a, sameName)
    {
        const r = await a.request.post('/api/v1/persons', {
            headers: { 'X-Family-Id': fam1, 'content-type': 'application/json' },
            data: { given_name: 'Alice', family_name: 'Mueller' },
        })
        expect(r.ok()).toBeTruthy()
    }

    // Owner C creates Müller #2 — same name — and person "BobC-{stamp}" in it.
    const cCtx = await browser.newContext()
    const c = await cCtx.newPage()
    await signIn(c, `edge-c-${stamp}@example.com`)
    const fam2 = await createFamily(c, sameName)
    {
        const r = await c.request.post('/api/v1/persons', {
            headers: { 'X-Family-Id': fam2, 'content-type': 'application/json' },
            data: { given_name: 'Bob', family_name: 'Mueller' },
        })
        expect(r.ok()).toBeTruthy()
    }

    // Guest B signs in (existing account, no families), then A invites B to
    // fam1, B accepts → B is in fam1. Then C invites B to fam2, B accepts →
    // B is in BOTH. The second accept rotates the access cookie + setActive's
    // fam2 client-side, so B's active family is the *token's* family (fam2).
    const bCtx = await browser.newContext()
    const b = await bCtx.newPage()
    const bEmail = `edge-b-${stamp}@example.com`
    await signIn(b, bEmail)
    await inviteAndAccept(a, b, fam1, bEmail, 'user')
    await expect(b.getByTestId('family-switcher')).toContainText(sameName, { timeout: 15_000 })
    await inviteAndAccept(c, b, fam2, bEmail, 'user')

    // Active family is fam2 (the most recent accept). Tree shows BobC, NOT AliceA.
    await expect(b.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Bob' }).first()).toBeVisible({
        timeout: 15_000,
    })
    await expect(b.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Alice' })).toHaveCount(0)

    // Switch to fam1 via the active-family store (the switcher's onChange
    // path; clicking the v-select overlay is rendering-engine-dependent and
    // adds noise to a logic test). The store-driven path invalidates queries,
    // which re-issues the tree fetch with the new X-Family-Id.
    await b.evaluate((id) => {
        localStorage.setItem('my-fam-tree:activeFamily', id)
    }, fam1)
    await b.goto('/tree')
    await expect(b.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Alice' }).first()).toBeVisible({
        timeout: 15_000,
    })
    await expect(b.locator('[data-testid^="tree-node-"]').filter({ hasText: 'Bob' })).toHaveCount(0)

    await bCtx.close()
    await cCtx.close()
    await aCtx.close()
})
