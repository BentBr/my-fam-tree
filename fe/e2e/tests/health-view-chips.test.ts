// HealthView surfaces two latency chips: the DB probe and the full
// handler duration. The BE always sets both fields; this e2e pins the
// FE rendering so a future refactor that drops the server chip surfaces
// here before users see it. Loosely parallel to the unit-level
// HealthView.test.ts; this test exercises the live BE response.

import { expect, test } from '../fixtures/console.fixture'

test('the health page renders both the database and server-latency chips', async ({ page }) => {
    // `/health` is mounted under MainLayout but its meta doesn't require
    // auth — visit it directly without signing in.
    await page.goto('/health')
    await expect(page.getByTestId('health-ok')).toBeVisible({ timeout: 10_000 })
    // Both chips render together on the success path.
    await expect(page.getByTestId('health-db')).toBeVisible()
    await expect(page.getByTestId('health-server')).toBeVisible()
    // The DB chip carries a "Database … ms" text; server carries
    // "Server … ms". The exact ms value is runner-dependent — we only
    // assert the kind label landed.
    await expect(page.getByTestId('health-db')).toContainText(/Database\s+\d+\s+ms/)
    await expect(page.getByTestId('health-server')).toContainText(/Server\s+\d+\s+ms/)
})
