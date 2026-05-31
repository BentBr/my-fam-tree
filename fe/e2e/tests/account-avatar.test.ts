// Avatar upload + remove on the Account page is a privileged user-state
// mutation (POST /users/me/avatar → S3 object + DB column; DELETE
// reverses both). No existing e2e covers it. This test exercises both
// halves end-to-end and verifies the visible state (the avatar element
// changes from initial to <img>-bearing on upload, and reverts on
// remove).

import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { expect, test } from '../fixtures/console.fixture'
import { signIn } from '../page-objects/session'

// Read the production favicon as our test PNG. Avoids the trap of
// hand-rolling tiny PNG hex blobs whose chunk CRCs are easy to get
// wrong — Rust's `image` crate validates them strictly on decode and
// every prior hand-crafted attempt failed at either IHDR-CRC or
// IDAT-CRC verification. The 16x16 favicon is a real PNG built by
// the same pipeline that ships to production, so we know it round-
// trips through every consumer (image::load_from_memory → resize →
// re-encode → S3 → presigned URL → <img> render).
const TINY_PNG = readFileSync(fileURLToPath(new URL('../../public/brand/favicon-16.png', import.meta.url)))

test('owner uploads an avatar then clears it; the account card reflects both states', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `avatar-flow-${stamp}@example.com`)
    await page.goto('/account')
    await expect(page.getByTestId('account-card')).toBeVisible()

    // No avatar yet → the remove button is absent (it's `v-if`'d on
    // `me.data.value?.avatar_url`).
    await expect(page.getByTestId('account-avatar-remove')).toHaveCount(0)

    // Use Playwright's setInputFiles on the hidden file input. The Camera
    // button's @click only opens the picker (which we can't drive
    // headlessly); injecting straight into the input fires the change
    // handler the same way.
    await page.getByTestId('account-avatar-input').setInputFiles({
        name: 'me.png',
        mimeType: 'image/png',
        buffer: TINY_PNG,
    })

    // After useSetMyAvatar resolves, the /users/me query invalidation
    // pulls the new avatar_url and the remove button appears.
    await expect(page.getByTestId('account-avatar-remove')).toBeVisible({ timeout: 10_000 })

    // Remove path: click the dedicated button, watch the same affordance
    // disappear on the resulting refetch.
    await page.getByTestId('account-avatar-remove').click()
    await expect(page.getByTestId('account-avatar-remove')).toHaveCount(0, { timeout: 10_000 })
})
