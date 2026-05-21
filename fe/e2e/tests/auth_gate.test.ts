import { expect, type Page, test } from '@playwright/test'

import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Mirrors the helper in auth_flow.test.ts. Kept inline because Playwright's
// project layout doesn't share helpers across test files unless we extract
// them into a fixture module — the helper count is still small enough that
// duplication is cheaper than the abstraction.
async function signIn(page: Page, email: string): Promise<void> {
    await clearMailpit()
    const login = new LoginPage(page)
    await login.goto()
    await login.signIn(email)
    await expect(login.sent).toBeVisible()
    const mail = await waitForEmail((s) => /Sign in to my-family|Anmeldung bei my-family/.test(s))
    const match = mail.text.match(/https?:\/\/\S+\/auth\/consume\?token=\S+/)
    if (match === null) {
        throw new Error('consume link not in email body')
    }
    const link = match[0]
    if (link === undefined) {
        throw new Error('consume link match was empty')
    }
    await page.goto(link)
    // After ConsumeView, the family guard sends a brand-new user (no families)
    // to /families/create. Either landing spot is fine — we just want past the
    // auth wall so the sign-out button is rendered by MainLayout's AppBar.
    await expect(page).toHaveURL(/\/(health|families\/create|families\/pick)$/)
}

test.describe('FE auth gate', () => {
    test('anonymous visit to /health redirects to /auth/sign-in', async ({ page }) => {
        await page.goto('/health')
        await expect(page).toHaveURL(/\/auth\/sign-in$/)
        await expect(page.getByTestId('login-card')).toBeVisible()
    })

    test('anonymous visit to /families/create redirects to /auth/sign-in', async ({ page }) => {
        await page.goto('/families/create')
        await expect(page).toHaveURL(/\/auth\/sign-in$/)
    })

    test('anonymous visit to / redirects to /auth/sign-in', async ({ page }) => {
        await page.goto('/')
        await expect(page).toHaveURL(/\/auth\/sign-in$/)
    })

    test('anonymous visit to /invite/accept (no token) lands on InviteAccept', async ({ page }) => {
        // With the /invite/* exemption in the router guard, the anonymous user
        // is NOT bounced to sign-in. InviteAccept's onMounted then sees an
        // empty token and renders the error state. (When a token IS present,
        // InviteAccept itself stashes it to sessionStorage and bounces.)
        await page.goto('/invite/accept')
        await expect(page).toHaveURL(/\/invite\/accept$/)
        await expect(page.getByTestId('invite-error')).toBeVisible()
    })

    test('logout from app-bar returns to /auth/sign-in', async ({ page }) => {
        // Sign in with a fresh email; family guard bounces to /families/create
        // but MainLayout (and therefore AppBar with the user menu) is still
        // rendered.
        await signIn(page, 'gate-test@example.com')

        // T6 moved sign-out behind a user-icon dropdown — open it first.
        await page.getByTestId('user-menu').click()
        const signOut = page.getByTestId('sign-out')
        await expect(signOut).toBeVisible()
        await signOut.click()

        await expect(page).toHaveURL(/\/auth\/sign-in$/)
        await expect(page.getByTestId('login-card')).toBeVisible()

        // Re-visiting a protected route must still bounce — confirms the
        // store really is back to anonymous, not just a one-shot navigation.
        await page.goto('/health')
        await expect(page).toHaveURL(/\/auth\/sign-in$/)
    })
})
