import type { Locator, Page } from '@playwright/test'

export class HealthPage {
    readonly ok: Locator
    readonly loading: Locator
    readonly error: Locator

    constructor(private readonly page: Page) {
        this.ok = page.getByTestId('health-ok')
        this.loading = page.getByTestId('health-loading')
        this.error = page.getByTestId('health-error')
    }

    async goto(): Promise<void> {
        await this.page.goto('/health')
    }
}
