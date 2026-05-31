// Picking a language from the LanguageMenu writes through to the
// locale store + persists across reloads (the auth store mirrors the
// chosen locale to localStorage; the BE's /users/me update keeps the
// signed-in user's preference). This e2e pins the round-trip so a
// future change to the menu or store can't silently drop the
// persistence.

import { expect, test } from '../fixtures/console.fixture'

test('selecting a language from the chrome menu survives a page reload', async ({ page }) => {
    // Anonymous-public flow — the LanguageMenu mounts in the top-bar
    // on every layout, including PublicLayout. No sign-in needed.
    await page.goto('/')
    await page.getByTestId('language-menu').click()
    // The menu reveals one item per supported locale; pick German.
    await page.getByTestId('language-menu-de').click()

    // The page title / nav re-renders under the German catalog. Pick a
    // stable label only present in German to verify the switch landed.
    // The footer's "Impressum" link is German-only; English shows
    // "Imprint". Look for the footer nav by aria-label.
    await expect(page.getByRole('link', { name: 'Impressum' })).toBeVisible()

    // Reload — the persisted locale comes back from localStorage at
    // bootstrap, so the German label stays.
    await page.reload()
    await expect(page.getByRole('link', { name: 'Impressum' })).toBeVisible()
})
