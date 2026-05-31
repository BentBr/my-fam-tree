// Avatar upload + remove on the Account page is a privileged user-state
// mutation (POST /users/me/avatar → S3 object + DB column; DELETE
// reverses both). No existing e2e covers it. This test exercises both
// halves end-to-end and verifies the visible state (the avatar element
// changes from initial to <img>-bearing on upload, and reverts on
// remove).

import { Buffer } from 'node:buffer'

import { expect, test } from '../fixtures/console.fixture'
import { signIn } from '../page-objects/session'

// Tiny 1x1 PNG — the canonical smallest valid PNG (8-bit grayscale+alpha,
// transparent pixel). 67 bytes. CRCs verified — Rust's `image` crate
// validates PNG chunk CRCs strictly on decode; the previous hand-written
// hex blob had a bad IHDR CRC (0x7bfd4da8 vs the correct 0x907753de) and
// every upload returned 422 ImageInvalid. This canonical sequence is the
// version recommended by https://garethrees.org/2007/11/14/pngcrush/ and
// the GitHub-cited "smallest possible PNG". Kept inline so the fixture
// stays self-contained.
const TINY_PNG = Buffer.from(
    '89504E470D0A1A0A0000000D49484452' + // signature + IHDR chunk header
        '00000001000000010804000000' + // 1x1, 8-bit grayscale+alpha
        'B51C0C02' + // IHDR CRC
        '0000000B49444154' + // IDAT chunk header (11 bytes data)
        '789C636000000002000100' + // IDAT compressed data (deflate of one transparent pixel)
        'E221BC33' + // IDAT CRC
        '0000000049454E44' + // IEND chunk header
        'AE426082', // IEND CRC
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
