import { defineConfig, devices, type PlaywrightTestConfig } from '@playwright/test'

// Default to the dinghy-routed local domain. CI overrides via E2E_BASE_URL.
const baseURL = process.env['E2E_BASE_URL'] ?? 'http://my-family.docker'
const isCI = process.env['CI'] !== undefined

// Local dev convenience: spin up `pnpm dev` if no server is reachable.
// CI brings up the full compose stack instead — the fe service serves the
// SPA on http://my-family.docker via dinghy.
const webServer: PlaywrightTestConfig['webServer'] = isCI
    ? []
    : {
          command: 'pnpm dev',
          url: baseURL,
          reuseExistingServer: true,
          timeout: 30_000,
      }

export default defineConfig({
    testDir: '.',
    // E2E tests share a single Mailpit inbox + Postgres + Redis. Running them in
    // parallel causes races on inbox state and family/user fixtures. Component
    // tests stay parallel because they're pure browser-render.
    fullyParallel: false,
    workers: 1,
    forbidOnly: isCI,
    retries: isCI ? 2 : 0,
    reporter: isCI ? [['github'], ['html', { open: 'never' }]] : 'list',
    use: {
        baseURL,
        trace: 'retain-on-failure',
        screenshot: 'only-on-failure',
        viewport: { width: 1440, height: 900 },
    },
    projects: [
        {
            name: 'e2e',
            testDir: './tests',
            fullyParallel: false,
            use: { ...devices['Desktop Chromium'] },
        },
    ],
    webServer,
})
