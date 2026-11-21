import { expect, test } from '../fixtures/console.fixture'
import { signIn } from '../page-objects/session'

// Input that can't be parsed (wrong type, impossible date, broken JSON, bad
// query string) must come back as the RFC 7807 problem+json envelope the FE
// error mapper understands, not as actix's plain-text 400.
test('unparseable requests return a problem+json 400 with request_malformed', async ({ page }) => {
    await signIn(page, `api-errors-${Date.now()}@example.com`)

    const wrongType = await page.request.put('/api/v1/reminder-preferences', {
        headers: { 'content-type': 'application/json' },
        data: {
            emails_enabled: 'yes',
            remind_birthdays: true,
            remind_anniversaries: true,
            favourites_only: false,
            lead_days: 7,
        },
    })
    expect(wrongType.status()).toBe(400)
    expect(wrongType.headers()['content-type']).toContain('application/problem+json')
    const body = (await wrongType.json()) as { code: string; status: number; detail: string }
    expect(body.code).toBe('request_malformed')
    expect(body.status).toBe(400)
    expect(body.detail).toMatch(/^malformed request body/)

    const brokenJson = await page.request.put('/api/v1/reminder-preferences', {
        headers: { 'content-type': 'application/json' },
        data: '{not json',
    })
    expect(brokenJson.status()).toBe(400)
    expect(((await brokenJson.json()) as { code: string }).code).toBe('request_malformed')
})
