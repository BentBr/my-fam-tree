import { expect, test } from '../fixtures/console.fixture'
import { signIn, createFamily } from '../page-objects/session'

// Phase 5 Task 20 — mobile responsiveness smoketest.
//
// On a phone-sized viewport (iPhone 12/13/14 logical size) two things must hold:
//   1. The primary nav drawer auto-collapses to a hidden temporary overlay
//      (it does NOT eat the viewport like the desktop permanent drawer) and is
//      revealed/dismissed by the app-bar hamburger.
//   2. The "Family tree of <name>" heading stays on-screen — it shrinks a step
//      and truncates rather than forcing the page to scroll horizontally, even
//      with a deliberately long family name.
//
// Desktop behaviour is covered by the existing tree.test.ts at the default
// 1440×900 viewport; here we explicitly shrink to 390×844 first.
const MOBILE = { width: 390, height: 844 } as const

test.describe('mobile responsiveness', () => {
    test('nav drawer collapses to a hamburger-toggled overlay and the tree heading does not overflow', async ({
        page,
    }) => {
        await page.setViewportSize(MOBILE)

        const stamp = Date.now()
        await signIn(page, `mobile-${stamp}@example.com`)
        // A long family name is the worst case for the heading: "Family tree of
        // <name>" must still not push the toolbar past the viewport edge.
        const longName = `Mobile Responsiveness Verification Family ${stamp}`
        await createFamily(page, longName)
        await expect(page).toHaveURL(/\/tree$/)

        // --- Part 1: drawer is collapsed (overlay hidden) by default ----------
        const drawer = page.getByTestId('nav-drawer')
        const treeNavLink = drawer.getByRole('link', { name: /Tree|Stammbaum/ })
        // Vuetify's temporary drawer stays in the DOM but slides off-canvas via
        // `transform: translateX(-100%)` rather than display:none, so a plain
        // `toBeHidden()` would (wrongly) report it visible. We instead assert
        // the drawer panel is parked off the left edge (right <= 0). A scrim is
        // only present while the overlay is open, so it is the cleanest open/
        // closed signal.
        const drawerRight = (): Promise<number> => drawer.evaluate((el) => el.getBoundingClientRect().right)
        expect(await drawerRight()).toBeLessThanOrEqual(0)
        await expect(page.locator('.v-navigation-drawer__scrim')).toHaveCount(0)

        // The hamburger reveals the overlay drawer: it slides on-canvas (left
        // edge at 0) and a dimming scrim appears over the content.
        await page.getByTestId('nav-toggle').click()
        await expect(page.locator('.v-navigation-drawer__scrim')).toHaveCount(1)
        await expect.poll(drawerRight).toBeGreaterThan(0)
        await expect(treeNavLink).toBeVisible()

        // Picking a destination dismisses the overlay again (good mobile UX):
        // the scrim disappears and the panel parks back off-canvas.
        await treeNavLink.click()
        await expect(page.locator('.v-navigation-drawer__scrim')).toHaveCount(0)
        await expect.poll(drawerRight).toBeLessThanOrEqual(0)

        // --- Part 2: the tree heading is present and does not overflow --------
        const title = page.getByTestId('tree-page-title')
        await expect(title).toBeVisible()
        // The DOM text is the full localized heading (CSS ellipsis truncates the
        // *rendered* glyphs, not the text content) — so we match the whole name.
        await expect(title).toHaveText(new RegExp(`(Family tree of|Stammbaum von) ${longName}`))

        // The heading must not spill past the viewport, and it must truncate
        // (clipped render width < natural text width) rather than overflow. The
        // ellipsis lives on Vuetify's inner `__placeholder` element, so that is
        // where `scrollWidth > clientWidth` shows the clamp.
        const overflow = await title.evaluate((el) => {
            const vw = window.innerWidth
            const rect = el.getBoundingClientRect()
            const placeholder = el.querySelector('.v-toolbar-title__placeholder') ?? el
            return {
                widerThanViewport: rect.right > vw + 1,
                truncates: placeholder.scrollWidth > placeholder.clientWidth,
            }
        })
        expect(overflow.widerThanViewport).toBe(false)
        expect(overflow.truncates).toBe(true)

        // And there is no horizontal page scroll on the mobile viewport.
        const horizontallyScrollable = await page.evaluate(
            () => document.documentElement.scrollWidth > document.documentElement.clientWidth + 1,
        )
        expect(horizontallyScrollable).toBe(false)
    })
})
