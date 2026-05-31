// HealthView surfaces three liveness chips: the DB probe, the worker
// leader-lease, and the full handler duration. The BE always sets the
// fields; this e2e pins the FE rendering so a future refactor that
// drops a chip surfaces here before users see it.

import { expect, test } from '../fixtures/console.fixture'
import { signIn } from '../page-objects/session'

test('the health page renders DB + worker + server-latency chips', async ({ page }) => {
    // `/health` lives in MainLayout and is gated by the family-active
    // guard (the route guard bounces anonymous visitors to /auth/sign-in
    // even though `requiresAuth: false` on the meta, since the route
    // isn't marked `public`). Sign in first.
    const stamp = Date.now()
    await signIn(page, `health-chips-${stamp}@example.com`)

    await page.goto('/health')
    await expect(page.getByTestId('health-ok')).toBeVisible({ timeout: 10_000 })
    // All three chips render together on the success path.
    await expect(page.getByTestId('health-db')).toBeVisible()
    await expect(page.getByTestId('health-worker')).toBeVisible()
    await expect(page.getByTestId('health-server')).toBeVisible()
    // Each chip carries a kind-specific text shape. Exact ms values are
    // runner-dependent — we only assert the label landed.
    await expect(page.getByTestId('health-db')).toContainText(/Database\s+\d+\s+ms/)
    await expect(page.getByTestId('health-server')).toContainText(/Server\s+\d+\s+ms/)
    // Worker chip says either "Worker alive" or "Worker down" depending
    // on whether the lease is currently held; both shapes are valid for
    // the test as long as the chip rendered.
    await expect(page.getByTestId('health-worker')).toContainText(/Worker/)
})
