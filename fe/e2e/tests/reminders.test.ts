import type { Page } from '@playwright/test'
import { expect, test } from '../fixtures/console.fixture'

import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { LoginPage } from '../page-objects/login.page'

// The worker exposes POST /__test/advance-clock only when built with
// `--features test-fixtures` (CI + `REMINDER_WORKER_FEATURES=test-fixtures`
// locally). In CI it listens on localhost:9091; under compose it's the
// `reminder-worker` service. Override via WORKER_TEST_URL.
const WORKER_URL = process.env.WORKER_TEST_URL ?? 'http://localhost:9091'

async function signIn(page: Page, email: string): Promise<void> {
    await clearMailpit()
    const login = new LoginPage(page)
    await login.goto()
    await login.signIn(email)
    await expect(login.sent).toBeVisible()
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

test('a daily digest email fires 7 days before a birthday when reminders are on', async ({ page }) => {
    const stamp = Date.now()
    await signIn(page, `reminders-${stamp}@example.com`)
    await createFamily(page, `Reminders-${stamp}`)

    const familyId = await page.evaluate(() => localStorage.getItem('my-family:activeFamily') ?? '')
    expect(familyId).not.toBe('')

    // Birthday 2026-06-15. With the worker clock advanced to 2026-06-08 (below)
    // and the default 7-day lead, the next occurrence (2026-06-15) is exactly
    // the target, so this person fires.
    const res = await page.request.post('/api/v1/persons', {
        headers: { 'X-Family-Id': familyId },
        data: { given_name: 'Birthday', family_name: 'Person', birth_date: '1990-06-15' },
    })
    expect(res.ok()).toBeTruthy()

    // Enable reminder emails via the Account panel (defaults: both kinds on,
    // lead_days 7). This exercises the FE preferences flow end-to-end. The
    // Vuetify v-switch toggles via its (visually hidden) input, so force-click
    // it rather than the decorative root.
    await page.goto('/account')
    await expect(page.getByTestId('reminder-prefs')).toBeVisible()
    await page.getByTestId('reminder-emails-enabled').locator('input').click({ force: true })
    await page.getByTestId('reminder-save').click()
    // Deterministically confirm the PUT persisted before advancing the clock
    // (more robust than racing the auto-dismissing toast).
    await expect
        .poll(
            async () => {
                const r = await page.request.get('/api/v1/reminder-preferences')
                if (!r.ok()) return false
                const body = (await r.json()) as { data: { emails_enabled: boolean } }
                return body.data.emails_enabled
            },
            { timeout: 10_000 },
        )
        .toBe(true)

    // Fast-forward the worker to 06:00 Europe/Berlin on 2026-06-08
    // (= 04:00 UTC, CEST). The advance-clock endpoint runs one tick
    // immediately; the dispatcher then sends the queued digest.
    await clearMailpit()
    const advance = await page.request.post(`${WORKER_URL}/__test/advance-clock`, {
        headers: { 'content-type': 'application/json' },
        data: { to: '2026-06-08T04:00:00Z' },
    })
    expect(advance.ok(), 'worker advance-clock should succeed').toBeTruthy()

    // Within the dispatcher's poll window the digest email lands in Mailpit.
    const digest = await waitForEmail((s) => /In 7 days|In 7 Tagen|🎂/.test(s), 30_000)
    expect(digest.text).toMatch(/Birthday Person/)
})
