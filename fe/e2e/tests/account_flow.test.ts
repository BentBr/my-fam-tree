import { expect, type Page, test } from '@playwright/test'

import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// Mirrors the helper in auth_flow.test.ts / auth_gate.test.ts. Playwright test
// files are independent modules — sharing a helper would require extracting it
// to a fixture; with three uses it's still cheaper to inline.
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
    await expect(page).toHaveURL(/\/(health|families\/create|families\/pick)$/)
}

async function createFamily(page: Page, name: string): Promise<void> {
    await page.goto('/families/create')
    await page.getByTestId('family-name').locator('input').fill(name)
    await page.getByTestId('family-create-submit').click()
    await expect(page).toHaveURL(/\/health$/)
}

test.describe('FE account flow', () => {
    test('user can update display name and locale', async ({ page }) => {
        await signIn(page, 'profile@example.com')
        await createFamily(page, 'Profile-Test')

        // Open the user menu and navigate to /account.
        await page.getByTestId('user-menu').click()
        await page.getByTestId('user-menu-account').click()
        await expect(page).toHaveURL(/\/account$/)
        await expect(page.getByTestId('account-card')).toBeVisible()

        // Wait for /users/me to populate the form, then overwrite.
        const nameInput = page.getByTestId('account-display-name').locator('input')
        await expect(nameInput).toBeEnabled()
        await nameInput.fill('Anna Müller')

        // v-select renders a hidden <input>; clicking the wrapper opens the
        // overlay where v-list-item titles are picked by visible text.
        await page.getByTestId('account-locale').click()
        await page.getByRole('option', { name: 'German' }).click()

        await page.getByTestId('account-save').click()

        // Reload and verify the backend really persisted the change.
        await page.reload()
        await expect(nameInput).toHaveValue('Anna Müller')
        // Locale select shows the currently selected title — "German" because
        // i18n was just switched to de? After PATCH the locale store flips, the
        // UI re-renders in German, so the option title is "Deutsch". The
        // underlying value is still "de" — assert via the model attribute.
        const localeInput = page.getByTestId('account-locale').locator('input').first()
        await expect(localeInput).toHaveValue('de')
    })

    test('email change roundtrip', async ({ page }) => {
        await signIn(page, 'change@example.com')
        await createFamily(page, 'Change-Test')

        await page.goto('/account')
        await expect(page.getByTestId('account-email-current')).toHaveText('change@example.com')

        // Clear mailpit so the next waitForEmail matches the email-change
        // notification, not the magic-link we just used to sign in.
        await clearMailpit()
        await page.getByTestId('account-email-new').locator('input').fill('change-new@example.com')
        await page.getByTestId('account-email-change-submit').click()
        await expect(page.getByTestId('email-change-pending')).toBeVisible()

        const mail = await waitForEmail((s) => /Confirm your email change|Bestätige deine E-Mail-Änderung/.test(s))
        const linkMatch = mail.text.match(/https?:\/\/\S+\/account\/email-change\/consume\?token=\S+/)
        if (linkMatch === null) {
            throw new Error('email-change link not in body')
        }
        const link = linkMatch[0]
        if (link === undefined) {
            throw new Error('email-change link match was empty')
        }

        await page.goto(link)
        // EmailChangeConsumeView routes back to /account after the confirm
        // mutation resolves. Accept the landing spot directly.
        await expect(page).toHaveURL(/\/account$/)

        // Reload to refetch /users/me from the server and verify the new email
        // really stuck (not just an optimistic store update).
        await page.reload()
        await expect(page.getByTestId('account-email-current')).toHaveText('change-new@example.com')
    })

    test('manual sign-out clears storage and redirects', async ({ page }) => {
        await signIn(page, 'logout-clear@example.com')
        await createFamily(page, 'Logout-Test')

        // applyClaimsPayload mirrored the user locale into the locale store,
        // which the store's bindToI18n watcher persists into localStorage.
        // Trigger a deliberate write so the test does not race the watcher.
        await page.evaluate(() => {
            localStorage.setItem('my-family:locale', 'en')
            sessionStorage.setItem('my-family:probe', '1')
        })

        await page.getByTestId('user-menu').click()
        await page.getByTestId('sign-out').click()
        await expect(page).toHaveURL(/\/auth\/sign-in$/)

        const localKeys = await page.evaluate(() => Object.keys(localStorage).filter((k) => k.startsWith('my-family:')))
        const sessionKeys = await page.evaluate(() =>
            Object.keys(sessionStorage).filter((k) => k.startsWith('my-family:')),
        )
        expect(localKeys).toEqual([])
        expect(sessionKeys).toEqual([])
    })
})
