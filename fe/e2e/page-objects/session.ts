import { type Page, expect } from '@playwright/test'

import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'

import { LoginPage } from './login.page'

/**
 * Magic-link sign-in: request the link, pull it from Mailpit, consume it, and
 * land on the post-auth route. Shared by the e2e suite (previously inlined in
 * ~18 spec files).
 */
export async function signIn(page: Page, email: string): Promise<void> {
    await clearMailpit()
    const login = new LoginPage(page)
    await login.goto()
    await login.signIn(email)
    await expect(login.sent).toBeVisible()
    const mail = await waitForEmail((s) => /Sign in to my-fam-tree|Anmeldung bei my-fam-tree/.test(s))
    const match = mail.text.match(/https?:\/\/\S+\/auth\/consume\?token=\S+/)
    if (match === null) throw new Error('consume link not in email body')
    const link = match[0]
    if (link === undefined) throw new Error('consume link match was empty')
    await page.goto(rewriteEmailLink(link))
    // ConsumeView's redirect chain is POST /auth/consume → applyClaimsPayload
    // → router.replace → beforeEach guards (hydrate skipped, active-family
    // resolve, role check) → final URL. Under CI runner load this can take
    // past Playwright's default 5 s `toHaveURL` poll window — bumping the
    // budget to 15 s leaves the redirect plenty of room without masking a
    // genuine hang (per-test timeout is still 30 s).
    await expect(page).toHaveURL(/\/(tree|health|families\/create|families\/pick)$/, { timeout: 15_000 })
}

/**
 * Create a family (the caller becomes its owner) and return the new family id
 * (read from the active-family store). Callers that don't need the id can
 * ignore the return value.
 */
export async function createFamily(page: Page, name: string): Promise<string> {
    await page.goto('/families/create')
    await page.getByTestId('family-name').locator('input').fill(name)
    await page.getByTestId('family-create-submit').click()
    await expect(page).toHaveURL(/\/tree$/)
    return page.evaluate(() => localStorage.getItem('my-fam-tree:activeFamily') ?? '')
}
