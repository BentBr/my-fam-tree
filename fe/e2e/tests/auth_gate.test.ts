import { expect, test } from '../fixtures/console.fixture'
import { signIn } from '../page-objects/session'

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
