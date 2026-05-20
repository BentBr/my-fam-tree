import { expect, test } from '@playwright/test'

import { HealthPage } from '../page-objects/health.page'

test('Health view shows the API is healthy', async ({ page }) => {
    const health = new HealthPage(page)
    await health.goto()
    await expect(health.ok).toBeVisible()
    await expect(page.getByText(/Version/)).toBeVisible()
    await expect(page.getByText(/Request id:|Request-ID:/)).toBeVisible()
})

test('Language switcher updates the title', async ({ page }) => {
    const health = new HealthPage(page)
    await health.goto()
    await expect(page.locator('.v-card-title')).toContainText('Service health')
    // Vuetify select: click the field, then the option.
    await page.locator('.v-app-bar .v-select').first().click()
    await page.getByRole('option', { name: /German|Deutsch/ }).click()
    await expect(page.locator('.v-card-title')).toContainText('Service-Status')
})
