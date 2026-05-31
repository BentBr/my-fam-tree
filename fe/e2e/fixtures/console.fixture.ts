import { test as baseTest, expect, type ConsoleMessage } from '@playwright/test'

import { flushRedis } from './redis.fixture'

// Reset the BE's per-IP rate-limit buckets before each test. CI runs all
// requests from 127.0.0.1; the api's `rate_limit_ip` cap (120/hour at
// `/auth/consume`, `/auth/refresh`, `/invite/accept`,
// `/owner-transfer/confirm`) accumulates across the suite and a fresh
// test that legitimately needs to refresh ends up getting a 429 that
// looks like a session-expiry bug. Same `FLUSHDB` helper as
// `global-setup`. Gated on `CI === 'true'` (or the explicit opt-in
// env var) so local dev runs don't slow down per-test.
const flushBetweenTests = process.env['CI'] === 'true' || process.env['E2E_FLUSH_REDIS'] === 'true'

// Console messages we explicitly tolerate. Each entry is a substring match
// against the raw text — if it's in here, we don't fail. Keep this list
// small and reviewed; the goal is zero noise, not zero failure.
const ALLOWLIST: readonly string[] = [
    // Vite dev tooling banner on first page load. CI uses the production
    // build, so this only fires locally — keep the allowlist sympathetic.
    'Download the Vue Devtools extension',
    // Vue Router emits an "uncaught" deprecation note for `[Vue Router warn]`
    // wrapped messages — these surface through `console.warn` AND through
    // `pageerror` simultaneously and we already gate on `console.warn`.
]

function isAllowed(text: string): boolean {
    return ALLOWLIST.some((pat) => text.includes(pat))
}

/**
 * Playwright fixture that attaches a `console`+`pageerror` listener to
 * every page and fails the test if any disallowed warning / error
 * surfaces during its run. Catches things like Vue Router "No match
 * found" misroutes and Vuetify "UPGRADE" deprecations that would
 * otherwise slip through manual testing.
 */
if (flushBetweenTests) {
    baseTest.beforeEach(async () => {
        await flushRedis()
    })
}

export const test = baseTest.extend<{ consoleErrors: string[] }>({
    consoleErrors: async ({ page }, use, testInfo) => {
        const errors: string[] = []
        const onConsole = (msg: ConsoleMessage): void => {
            const level = msg.type()
            if (level !== 'warning' && level !== 'error') return
            const text = msg.text()
            if (isAllowed(text)) return
            errors.push(`[console.${level}] ${text}`)
        }
        const onPageError = (err: Error): void => {
            // Page errors that surface as exception messages
            // (uncaught throws / promise rejections in page scripts).
            const text = err.message
            if (isAllowed(text)) return
            errors.push(`[pageerror] ${text}`)
        }
        page.on('console', onConsole)
        page.on('pageerror', onPageError)
        await use(errors)
        page.off('console', onConsole)
        page.off('pageerror', onPageError)
        // Only assert when the test itself passed — a test that already
        // failed gets its console noise reported in the trace anyway, no
        // point double-failing it and burying the real message.
        if (testInfo.status === 'passed' && errors.length > 0) {
            const summary = errors.map((line) => `  - ${line}`).join('\n')
            expect.soft(errors, `Browser console emitted ${errors.length} entries:\n${summary}`).toEqual([])
        }
    },
})

export { expect }
