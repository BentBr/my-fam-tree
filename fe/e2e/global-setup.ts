// Drain the Redis instance shared by the api so each E2E run starts from a
// clean rate-limiter state. Without this, repeated runs against the same
// emails (`gate-test@example.com`, `profile@example.com`, etc.) eventually
// trip `MAGIC_LINK_RATE_PER_EMAIL_PER_HOUR=5` and the magic-link send returns
// 429 — the page never reaches its "Check your inbox" state and every signIn
// helper fails. Mailpit is cleared per-test by `clearMailpit()`.
//
// The same FLUSHDB helper is invoked per-test from
// `fixtures/console.fixture.ts` so the BE's per-IP rate caps (120/hour at
// `auth/consume`, `auth/refresh`, `invite/accept`, `owner-transfer/confirm`)
// don't accumulate across the 62-test suite when everything shares
// 127.0.0.1 in CI.

import { flushRedis } from './fixtures/redis.fixture'

// Best-effort worker-clock reset. The worker's OffsetClock is process-state:
// a fast-forward from a PREVIOUS run/attempt (the reminders digest test, or
// a stray advance-clock probe) survives into this run and poisons it — the
// janitor then sweeps with a far-future cutoff and DELETEs freshly-minted
// single-use tokens, so sign-ins intermittently 401 with "link may have
// expired" from the very first test. Resetting to real "now" keeps the api
// and worker agreeing on time for the whole suite. Silently skipped when the
// worker wasn't built with `test-fixtures` (listener absent / unreachable).
async function resetWorkerClock(): Promise<void> {
    const workerUrl = process.env['WORKER_TEST_URL'] ?? 'http://worker.my-fam-tree.docker:9091'
    try {
        const res = await fetch(`${workerUrl}/__test/advance-clock`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ to: new Date().toISOString() }),
        })
        console.log(
            res.ok
                ? '[E2E Setup] worker clock reset to real now'
                : `[E2E Setup] worker clock reset skipped (HTTP ${String(res.status)})`,
        )
    } catch {
        console.log('[E2E Setup] worker clock reset skipped (test-fixtures listener unreachable)')
    }
}

export default async function globalSetup(): Promise<void> {
    if (process.env['CI'] === 'true' || process.env['E2E_FLUSH_REDIS'] === 'true') {
        await flushRedis()
        console.log('[E2E Setup] flushed redis (rate-limit buckets cleared)')
    } else {
        console.log('[E2E Setup] skipping redis flush (set CI=true or E2E_FLUSH_REDIS=true to enable)')
    }
    await resetWorkerClock()
}
