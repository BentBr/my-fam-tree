import type { Page } from '@playwright/test'

import { expect, test } from '../fixtures/console.fixture'
import { createFamily, signIn } from '../page-objects/session'

/**
 * 1×1 transparent PNG — the smallest legal PNG. Playwright's
 * `setInputFiles` accepts a Buffer-backed file directly so we don't
 * need a fixture file on disk; this keeps the test self-contained and
 * cross-platform. Magic bytes (89 50 4E 47 …) match the BE's
 * `validate_and_resize` accept-list.
 */
const TINY_PNG_BYTES = Buffer.from(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=',
    'base64',
)

const TREE_NODE_UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

async function listTreeNodeIds(page: Page): Promise<string[]> {
    const raw = await page
        .locator('[data-testid^="tree-node-"]')
        .evaluateAll((els) => els.map((el) => (el.getAttribute('data-testid') ?? '').replace('tree-node-', '')))
    return raw.filter((id) => TREE_NODE_UUID_RE.test(id))
}

async function addPerson(page: Page, given: string, family: string): Promise<string> {
    const existing = await listTreeNodeIds(page)
    await page.getByTestId('tree-add-person').click()
    await page.getByTestId('person-given-name').locator('input').fill(given)
    await page.getByTestId('person-family-name').locator('input').fill(family)
    await page.getByTestId('person-submit').click()
    await expect(page.getByTestId('person-detail')).toBeVisible()
    await expect.poll(async () => (await listTreeNodeIds(page)).length, { timeout: 10_000 }).toBe(existing.length + 1)
    const ids = await listTreeNodeIds(page)
    const added = ids.find((id) => !existing.includes(id))
    if (added === undefined) throw new Error('could not resolve newly-added person id')
    return added
}

test('owner uploads a photo for a person; tree avatar swaps from initials to image and survives reload', async ({
    page,
}) => {
    await signIn(page, 'photo-owner@example.com')
    await createFamily(page, 'Photo')
    const id = await addPerson(page, 'Werner', 'Schmidt')

    // ----- Before upload: hero shows the initials fallback, tree node shows
    // the SVG initials text (no <image> child on the node group).
    await expect(page.getByTestId('person-detail-hero-fallback')).toBeVisible()
    await expect(page.getByTestId('person-detail-hero-photo')).toHaveCount(0)
    await expect(page.getByTestId(`tree-node-photo-${id}`)).toHaveCount(0)

    // ----- Upload. The camera button triggers the hidden file input; we
    // shortcut straight to setInputFiles which Playwright treats as a
    // direct selection on the input element regardless of visibility.
    await page
        .getByTestId('person-detail-photo-input')
        .setInputFiles({ name: 'werner.png', mimeType: 'image/png', buffer: TINY_PNG_BYTES })

    // ----- After upload: sidebar swaps to the image; tree node swaps to
    // the SVG <image>. The presigned URL is non-empty.
    await expect(page.getByTestId('person-detail-hero-photo')).toBeVisible({ timeout: 10_000 })
    const sidebarSrc = await page.getByTestId('person-detail-hero-photo').getAttribute('src')
    expect(sidebarSrc).not.toBeNull()
    expect((sidebarSrc ?? '').length).toBeGreaterThan(0)

    // Tree-node photo lands once the ['tree'] cache invalidation propagates.
    await expect(page.getByTestId(`tree-node-photo-${id}`)).toBeVisible({ timeout: 10_000 })

    // ----- Persistence: reload the page, photo is still there on both
    // surfaces (BE returns photo_key/photo_url freshly presigned).
    await page.reload()
    await expect(page.getByTestId(`tree-node-photo-${id}`)).toBeVisible({ timeout: 10_000 })

    // Re-open the drawer to assert the sidebar persisted too.
    await page.getByTestId(`tree-node-${id}`).dispatchEvent('click')
    await expect(page.getByTestId('person-detail')).toBeVisible()
    await expect(page.getByTestId('person-detail-hero-photo')).toBeVisible({ timeout: 10_000 })
})

test('owner removes a photo; tree node falls back to initials', async ({ page }) => {
    await signIn(page, 'photo-remove@example.com')
    await createFamily(page, 'PhotoRemove')
    const id = await addPerson(page, 'Karin', 'Hoffmann')

    // Upload then remove.
    await page
        .getByTestId('person-detail-photo-input')
        .setInputFiles({ name: 'karin.png', mimeType: 'image/png', buffer: TINY_PNG_BYTES })
    await expect(page.getByTestId(`tree-node-photo-${id}`)).toBeVisible({ timeout: 10_000 })

    await page.getByTestId('person-detail-photo-remove').click()

    // Sidebar reverts to the initials fallback; tree drops the <image>.
    await expect(page.getByTestId('person-detail-hero-fallback')).toBeVisible({ timeout: 10_000 })
    await expect(page.getByTestId('person-detail-hero-photo')).toHaveCount(0)
    await expect(page.getByTestId(`tree-node-photo-${id}`)).toHaveCount(0)
})
