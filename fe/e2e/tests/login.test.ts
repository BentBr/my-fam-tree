import { expect, test } from '@playwright/test'

import { LoginPage } from '../page-objects/login.page'

test('Login: requesting a magic link shows the success state', async ({ page }) => {
    const login = new LoginPage(page)
    await login.goto()
    await expect(login.card).toBeVisible()
    await login.signIn('phase-0d@example.com')
    // Phase 0d uses a stub mutation (no real email sent). Phase 1b will extend
    // this test with mailpit assertions once the magic-link endpoint exists.
    await expect(login.sent).toBeVisible()
})

test('Login: consume route handles a missing token gracefully', async ({ page }) => {
    await page.goto('/auth/consume')
    await expect(page.getByTestId('consume-error')).toBeVisible()
})
