import type { Page } from '@playwright/test'
import { expect, test } from '../fixtures/console.fixture'

import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// ---------------------------------------------------------------------------
// Shared bootstrap helpers. Mirrors tree.test.ts — inlined per the existing
// directory convention rather than extracted to a fixture, so each test in
// this file stays self-contained. The relationship_validations suite drives
// every hard rule (422) and the soft warning (200 + meta.warnings) flow
// added in Phase 5 / Task 11.
// ---------------------------------------------------------------------------

async function signIn(page: Page, email: string): Promise<void> {
    await clearMailpit()
    const login = new LoginPage(page)
    await login.goto()
    await login.signIn(email)
    // First magic-link POST in CI can lag >5s after a cold api start; the
    // visible "Check your inbox" panel renders as soon as the POST resolves.
    await expect(login.sent).toBeVisible({ timeout: 15_000 })
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

interface PersonOpts {
    given: string
    family: string
    birth?: string
    death?: string
}

// The outer node group is `<g data-testid="tree-node-<uuid>">`, but the
// SVG also emits sibling `<text>` nodes with non-UUID testids
// (`tree-node-name`, `tree-node-birth`, `tree-node-death`) for fine-grained
// assertions. A plain `[data-testid^="tree-node-"]` count therefore returns
// 2–4 hits per actual person. We pull all matches and filter to the UUID
// form in JS — works under `noUncheckedIndexedAccess` and is robust to
// future fine-grained testid additions inside TreeNode.
const TREE_NODE_UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

async function listTreeNodeIds(page: Page): Promise<string[]> {
    const raw = await page
        .locator('[data-testid^="tree-node-"]')
        .evaluateAll((els) => els.map((el) => (el.getAttribute('data-testid') ?? '').replace('tree-node-', '')))
    return raw.filter((id) => TREE_NODE_UUID_RE.test(id))
}

/**
 * Adds a person via the create drawer. Mirrors the helper in tree.test.ts
 * but extends it to optionally tick "deceased" and fill the death date —
 * required for the `parent_deceased_before_child` rule which needs a
 * parent with a death date predating the child's conception window.
 *
 * Returns the new person's id (the trailing UUID off `data-testid="tree-node-<id>"`).
 */
async function addPerson(page: Page, opts: PersonOpts): Promise<string> {
    const existingIds = await listTreeNodeIds(page)
    const expectedAfter = existingIds.length + 1

    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill(opts.given)
    await page.getByTestId('person-family-name').locator('input').fill(opts.family)
    if (opts.birth !== undefined) {
        await page.getByTestId('person-birth-date').locator('input').fill(opts.birth)
    }
    if (opts.death !== undefined) {
        // The deceased toggle gates the death-date input; flip it then fill.
        await page.getByTestId('person-deceased').locator('input').check()
        await page.getByTestId('person-death-date').locator('input').fill(opts.death)
    }
    await page.getByTestId('person-submit').click()

    await expect(page.getByTestId('person-detail')).toBeVisible()
    // Poll until the post-mutation tree refetch settles — the create
    // mutation invalidates the tree query, but the SVG re-render lands a
    // tick or two later. Filter to UUID-shaped ids each poll.
    await expect.poll(async () => (await listTreeNodeIds(page)).length, { timeout: 10_000 }).toBe(expectedAfter)
    const ids = await listTreeNodeIds(page)
    const added = ids.find((id) => !existingIds.includes(id))
    if (added === undefined) throw new Error('could not resolve newly-added person id')
    return added
}

async function closeDrawer(page: Page): Promise<void> {
    await page.getByTestId('person-detail-close').click()
    await expect(page.getByTestId('person-detail')).toBeHidden()
    // Wait for the v-navigation-drawer scrim to be fully gone so subsequent
    // clicks aren't intercepted by its fade-out transition.
    await expect(page.locator('.v-navigation-drawer__scrim')).toHaveCount(0)
}

async function clickTreeNode(page: Page, id: string): Promise<void> {
    // The SVG canvas pans/zooms with d3-zoom so the on-screen node position
    // is outside Playwright's CSS-viewport check. Programmatic clicks fire
    // the same Vue handler.
    await page.getByTestId(`tree-node-${id}`).dispatchEvent('click')
}

/**
 * Link `parentId` as a parent of `childId` with `kind`. Opens the child's
 * drawer, picks the parent, optionally changes the kind dropdown, submits.
 * Caller decides whether to assert success or failure afterwards.
 */
async function linkParent(
    page: Page,
    childId: string,
    parentName: string,
    kind: 'biological' | 'legal' | 'adoptive' | 'step' | 'social' = 'biological',
): Promise<void> {
    await clickTreeNode(page, childId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    // The "Add parent" v-select sits inside the Parents v-expansion-panel
    // which is collapsed by default — expand it first so the trigger
    // becomes pointer-clickable.
    await page.getByTestId('relations-parents').click()
    await page.getByTestId('person-add-parent').click()
    await page.getByRole('option', { name: parentName }).click()
    if (kind !== 'biological') {
        await page.getByTestId('person-add-parent-kind').click()
        await page
            .getByRole('option', { name: new RegExp(`^${kind.replace(/^./, (c) => c.toUpperCase())}`, 'i') })
            .click()
    }
    await page.getByTestId('person-add-parent-submit').click()
}

interface PartnerOpts {
    partnerName: string
    kind: 'marriage' | 'civil_union' | 'partnership'
    startedOn?: string
}

async function linkPartner(page: Page, personId: string, opts: PartnerOpts): Promise<void> {
    await clickTreeNode(page, personId)
    await expect(page.getByTestId('person-detail')).toBeVisible()
    // Same expansion-panel gating for the Partners section.
    await page.getByTestId('relations-partners').click()
    await page.getByTestId('person-add-partner').click()
    await page.getByRole('option', { name: opts.partnerName }).click()
    await page.getByTestId('person-add-partner-kind').click()
    const kindLabel =
        opts.kind === 'marriage'
            ? /Marriage|Ehe/
            : opts.kind === 'civil_union'
              ? /Civil union|Eingetragene Partnerschaft/
              : /^Partnership$|Partnerschaft/
    await page.getByRole('option', { name: kindLabel }).click()
    if (opts.startedOn !== undefined) {
        await page.getByTestId('person-add-partner-started-on').locator('input').fill(opts.startedOn)
    }
    await page.getByTestId('person-add-partner-submit').click()
}

/**
 * Wait for the first toast that contains `text`. Returns the toast locator
 * so callers can also assert on `data-testid-kind` etc. Uses `.first()`
 * because tree.refetch / other side effects can stack info toasts.
 */
async function expectToastContaining(
    page: Page,
    text: string | RegExp,
    kind?: 'error' | 'success' | 'info',
): Promise<void> {
    const toast = page.getByTestId('toast').first()
    await expect(toast).toBeVisible()
    await expect(toast).toContainText(text)
    if (kind !== undefined) {
        await expect(toast).toHaveAttribute('data-testid-kind', kind)
    }
}

// ---------------------------------------------------------------------------
// 422 hard rejections (validation.* codes → field violations → error toast).
// ---------------------------------------------------------------------------

test('422 rejects a parent born after the child (parent_not_older_than_child)', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `parent-age-${stamp}@example.com`)
    await createFamily(page, `ParentAge-${stamp}`)
    await page.goto('/tree')

    // Klaus (older, the would-be child here) + Tiny (born long after Klaus,
    // attempted as parent). The validator only cares about birth dates, not
    // role labels, so any pair where parent.birth >= child.birth trips it.
    const klausId = await addPerson(page, { given: 'Klaus', family: 'Müller', birth: '1965-04-22' })
    await closeDrawer(page)
    await addPerson(page, { given: 'Tiny', family: 'Test', birth: '2020-01-01' })
    await closeDrawer(page)

    await linkParent(page, klausId, 'Tiny Test')
    await expectToastContaining(page, 'A parent must be born before the child.', 'error')
})

test('422 caps biological parents at two (too_many_biological_parents)', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `too-many-${stamp}@example.com`)
    await createFamily(page, `TooMany-${stamp}`)
    await page.goto('/tree')

    // Klaus (G2, child) + two seeded bio parents (Otto + Hannelore) + one
    // intruder we'll try to add as a third. All older than Klaus so rule 1
    // doesn't accidentally fire first.
    const klausId = await addPerson(page, { given: 'Klaus', family: 'Müller', birth: '1965-04-22' })
    await closeDrawer(page)
    await addPerson(page, { given: 'Otto', family: 'Müller', birth: '1935-03-12' })
    await closeDrawer(page)
    await addPerson(page, { given: 'Hannelore', family: 'Müller', birth: '1938-07-23' })
    await closeDrawer(page)
    await addPerson(page, { given: 'Spare', family: 'Müller', birth: '1940-06-01' })
    await closeDrawer(page)

    // Wire up the two legitimate biological parents first; both should
    // succeed silently (no error toast appears).
    await linkParent(page, klausId, 'Otto Müller')
    await expect(page.locator('[data-testid="tree-edge-parent"]')).toHaveCount(1)
    await closeDrawer(page)
    await linkParent(page, klausId, 'Hannelore Müller')
    await expect(page.locator('[data-testid="tree-edge-parent"]')).toHaveCount(2)
    await closeDrawer(page)

    // Third bio parent: rejected.
    await linkParent(page, klausId, 'Spare Müller')
    await expectToastContaining(page, 'A child can have at most two biological parents.', 'error')
})

test('422 rejects partnership starting before either partner was born (partnership_before_birth)', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `partnership-pre-birth-${stamp}@example.com`)
    await createFamily(page, `PreBirth-${stamp}`)
    await page.goto('/tree')

    const linaId = await addPerson(page, { given: 'Lina', family: 'Müller', birth: '1995-12-03' })
    await closeDrawer(page)
    await addPerson(page, { given: 'Max', family: 'Müller', birth: '1998-04-17' })
    await closeDrawer(page)

    await linkPartner(page, linaId, {
        partnerName: 'Max Müller',
        kind: 'marriage',
        startedOn: '1990-01-01',
    })
    await expectToastContaining(page, 'A partnership cannot start before either partner was born.', 'error')
})

test('422 rejects biological parent who died before child was conceived (parent_deceased_before_child)', async ({
    page,
}) => {
    const stamp = Date.now()
    await signIn(page, `parent-deceased-${stamp}@example.com`)
    await createFamily(page, `Deceased-${stamp}`)
    await page.goto('/tree')

    const maxId = await addPerson(page, { given: 'Max', family: 'Müller', birth: '1998-04-17' })
    await closeDrawer(page)
    // Born long enough before Max that rule 1 doesn't fire; died in 1990 —
    // more than 280 days (~9mo gestation) before Max's 1998 birth.
    await addPerson(page, {
        given: 'Ghost',
        family: 'Parent',
        birth: '1900-01-01',
        death: '1990-01-01',
    })
    await closeDrawer(page)

    await linkParent(page, maxId, 'Ghost Parent')
    await expectToastContaining(page, 'A biological parent must be alive at conception', 'error')
})

// ---------------------------------------------------------------------------
// 409 conflicts (top-level code → errorCodes.* lookup → error toast).
// ---------------------------------------------------------------------------

test('409 rejects a duplicate partnership of the same kind (partnership_duplicate)', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `partnership-dup-${stamp}@example.com`)
    await createFamily(page, `Dup-${stamp}`)
    await page.goto('/tree')

    const klausId = await addPerson(page, { given: 'Klaus', family: 'Müller', birth: '1965-04-22' })
    await closeDrawer(page)
    await addPerson(page, { given: 'Anna', family: 'Müller', birth: '1968-08-11' })
    await closeDrawer(page)

    // First marriage: succeeds. The partner edge appearing on the canvas is
    // the success signal; no toast is pushed on the happy path here (the
    // mutation hook only invalidates the tree query — see useCreatePartnership).
    await linkPartner(page, klausId, { partnerName: 'Anna Müller', kind: 'marriage' })
    await expect(page.locator('[data-testid="tree-edge-partner"]').first()).toBeVisible()
    await closeDrawer(page)

    // Second attempt at the same kind: 409 PartnershipDuplicate.
    await linkPartner(page, klausId, { partnerName: 'Anna Müller', kind: 'marriage' })
    await expectToastContaining(
        page,
        'A current partnership of the same kind already exists for these partners.',
        'error',
    )
})

// ---------------------------------------------------------------------------
// 409 cycle detection (existing rule — re-confirmed end-to-end).
// ---------------------------------------------------------------------------

test('409 rejects a parent link that would close a cycle (relationship_cycle)', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `cycle-${stamp}@example.com`)
    await createFamily(page, `Cycle-${stamp}`)
    await page.goto('/tree')

    const ottoId = await addPerson(page, { given: 'Otto', family: 'Müller', birth: '1935-03-12' })
    await closeDrawer(page)
    const klausId = await addPerson(page, { given: 'Klaus', family: 'Müller', birth: '1965-04-22' })
    await closeDrawer(page)

    // Otto is Klaus's parent.
    await linkParent(page, klausId, 'Otto Müller')
    await expect(page.locator('[data-testid="tree-edge-parent"]')).toHaveCount(1)
    await closeDrawer(page)

    // Attempting to also make Klaus a parent of Otto closes the loop.
    // The route's would_create_cycle short-circuit returns ApiError::RelationshipCycle
    // (409, code = relationship_cycle). queryClient renders the errorCodes
    // entry directly since the body carries no field violations.
    await linkParent(page, ottoId, 'Klaus Müller')
    await expectToastContaining(page, 'That link would create a cycle in the family tree.', 'error')
})

// ---------------------------------------------------------------------------
// 200 + soft warning (warning.* → info toast via warningsBroadcaster).
// ---------------------------------------------------------------------------

test('200 with sibling_partnership warning surfaces an info toast and still writes the row', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `sibling-${stamp}@example.com`)
    await createFamily(page, `Sibling-${stamp}`)
    await page.goto('/tree')

    // Build the minimum siblings-of-the-same-parents graph: Klaus + Anna are
    // both parents of Lina + Max. Then attempt to partner Lina with Max —
    // warning.sibling_partnership fires (no hard rejection). Klaus/Anna ids
    // aren't needed once they're in the v-select item list (matched by name).
    await addPerson(page, { given: 'Klaus', family: 'Müller', birth: '1965-04-22' })
    await closeDrawer(page)
    await addPerson(page, { given: 'Anna', family: 'Müller', birth: '1968-08-11' })
    await closeDrawer(page)
    const linaId = await addPerson(page, { given: 'Lina', family: 'Müller', birth: '1995-12-03' })
    await closeDrawer(page)
    const maxId = await addPerson(page, { given: 'Max', family: 'Müller', birth: '1998-04-17' })
    await closeDrawer(page)

    await linkParent(page, linaId, 'Klaus Müller')
    await closeDrawer(page)
    await linkParent(page, linaId, 'Anna Müller')
    await closeDrawer(page)
    await linkParent(page, maxId, 'Klaus Müller')
    await closeDrawer(page)
    await linkParent(page, maxId, 'Anna Müller')
    await closeDrawer(page)
    // Sanity: 4 parent edges in place (Lina×2 + Max×2).
    await expect(page.locator('[data-testid="tree-edge-parent"]')).toHaveCount(4)

    // Klaus's own birth needs to predate the partnership start the linkParent
    // calls implicitly create — we left started_on unset, so the validator
    // doesn't constrain. The partnership itself goes through; the FE shows
    // the warning info toast translated from `warning.sibling_partnership`.
    await linkPartner(page, linaId, { partnerName: 'Max Müller', kind: 'partnership' })

    // Soft warning toast (info kind) — exact translated copy from en.json.
    await expectToastContaining(page, 'share at least one parent', 'info')

    // And the partnership row was still created — heart edge renders.
    await expect(page.locator('[data-testid="tree-edge-partner"]').first()).toBeVisible()
})
