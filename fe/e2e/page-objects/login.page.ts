import type { Locator, Page } from '@playwright/test'

export class LoginPage {
    readonly card: Locator
    readonly email: Locator
    readonly submit: Locator
    readonly sent: Locator
    readonly error: Locator

    constructor(private readonly page: Page) {
        this.card = page.getByTestId('login-card')
        this.email = page.getByTestId('sign-in-email')
        this.submit = page.getByTestId('sign-in-submit')
        this.sent = page.getByTestId('sign-in-sent')
        this.error = page.getByTestId('login-error')
    }

    async goto(): Promise<void> {
        await this.page.goto('/auth/sign-in')
    }

    async signIn(email: string): Promise<void> {
        await this.email.fill(email)
        await this.submit.click()
    }
}
