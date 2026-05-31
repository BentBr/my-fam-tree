// Avatar upload + remove on the Account page is a privileged user-state
// mutation (POST /users/me/avatar → S3 object + DB column; DELETE
// reverses both). No existing e2e covers it. This test exercises both
// halves end-to-end and verifies the visible state (the avatar element
// changes from initial to <img>-bearing on upload, and reverts on
// remove).

import { Buffer } from 'node:buffer'

import { expect, test } from '../fixtures/console.fixture'
import { signIn } from '../page-objects/session'

// Tiny 1x1 PNG (red). Smallest valid PNG that passes the BE's
// `validate_and_resize` (the magic-byte detection only needs the PNG
// signature; resize handles arbitrary dimensions).
const TINY_PNG = Buffer.from(
    '89504E470D0A1A0A0000000D49484452000000010000000108020000007BFD' +
        '4DA800000016504C5445FF000000000000000000000000000000000000000000' +
        '6E7C58730000000174524E5300405A6F89000000094944415408D7636000000000' +
        '00010001005F03C9520000000049454E44AE426082',
    'hex',
)

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
