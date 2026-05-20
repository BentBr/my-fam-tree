// Runs once before any E2E test in the suite. Phase 0d's login flow is a
// stubbed mutation — no persistent data is created yet, so this is currently
// just a marker. Phase 1+ will populate this with admin login + test data
// creation (see r_data_core/fe/e2e/global-setup.ts for the pattern).

export default function globalSetup(): void {
    console.log('[E2E Setup] Phase 0d: nothing to seed yet')
}
