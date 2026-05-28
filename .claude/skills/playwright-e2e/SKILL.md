---
name: playwright-e2e
description: Use when writing, running, or debugging Playwright end-to-end tests in fe/e2e, or when reproducing/inspecting a frontend bug in a real browser. Covers the dedicated playwright compose service, the magic-link/Mailpit + console-error + link-rewrite fixtures, global Redis-flush/DB-truncate setup, data-testid selectors, traces/reports, and live browser debugging via the Playwright MCP. Load for E2E flakiness, "test can't reach the app", or full-stack flow tests.
---

# Playwright E2E + live browser debugging

For component/unit tests and the FE stack, load `frontend-workflow`. For domain terms,
`project-concepts`. For the debugging *method*, **REQUIRED BACKGROUND:**
superpowers:systematic-debugging.

## Where E2E runs (and where it does NOT)

E2E runs against the **live compose stack**, inside the dedicated **`playwright`
service** (`mcr.microsoft.com/playwright:v1.60.0-noble`). It does **not** run in the
`fe` service — that's `node:alpine`/musl and can't execute the Chromium build.
`scripts/fe-in-container.sh` routes `test:e2e` / `playwright` / `exec` to the
playwright container automatically.

```bash
rdt start                                   # stack must be up first
rdt test-e2e                                # full suite (= fe-in-container.sh test:e2e)
scripts/fe-in-container.sh test:e2e --grep "sign in"   # subset by test title
scripts/fe-in-container.sh exec playwright test -c e2e/playwright.config.ts --debug
```

Inside the container `E2E_BASE_URL=http://my-family.docker:5173` and
`MAILPIT_URL=http://mail.my-family.docker:8025` (set in `compose.yaml`).

## Config & isolation (`fe/e2e/playwright.config.ts`)

- `baseURL = E2E_BASE_URL ?? http://my-family.docker`; project `e2e` = Desktop Chromium;
  viewport 1440×900; `trace: retain-on-failure`, `screenshot: only-on-failure`.
- **`fullyParallel: false`, `workers: 1`** on purpose — all tests share one Mailpit,
  Postgres, and Redis. Don't "fix" flakiness by enabling parallelism.
- `globalSetup` flushes Redis (clears rate-limit buckets); `globalTeardown` truncates
  application tables. So each *run* starts clean — but tests within a run are ordered
  and should clear Mailpit themselves.

## Fixtures & conventions (`fe/e2e/`)

| File | What it gives you |
|---|---|
| `fixtures/console.fixture.ts` | the `test` export — **import `test` from here, not `@playwright/test`**. It fails the test on unexpected `console.error`/`pageerror` (Vue Devtools allowlisted). |
| `fixtures/mailpit.fixture.ts` | `clearMailpit()`, `waitForEmail(matcher, timeoutMs)` |
| `fixtures/email-links.fixture.ts` | `rewriteEmailLink(link)` — maps the email's `http://my-family.docker` URL to the in-container `E2E_BASE_URL` |
| `page-objects/login.page.ts` | `LoginPage` (locators by `data-testid`: `login-card`, `sign-in-email`, `sign-in-submit`, `sign-in-sent`, `login-error`) |

**Selectors:** use `data-testid` (via `getByTestId`), not CSS/text. If a component
lacks a testid you need, add one to the component.

**Magic-link sign-in pattern:** `clearMailpit()` → fill `sign-in-email` + click
`sign-in-submit` → `waitForEmail(s => /Sign in|Anmeldung/.test(s))` → extract
`/auth/consume?token=…` from the body → `rewriteEmailLink(...)` → `page.goto(...)` →
assert redirect (`/tree`, `/families/create`, etc.). Tests are `*.test.ts` (never
`.spec.ts`).

## Artifacts

HTML report: `fe/playwright-report/index.html` (open on host). Failure traces +
screenshots: `fe/test-results/` — inspect a trace with
`scripts/fe-in-container.sh exec playwright show-trace test-results/<path>/trace.zip`.
Both dirs are gitignored.

## Live debugging with the Playwright MCP

For interactive reproduction (not regression), drive a **real browser via the
Playwright MCP** (`browser_navigate`, `browser_snapshot`, `browser_click`,
`browser_fill_form`, `browser_console_messages`, `browser_network_requests`,
`browser_take_screenshot`, `browser_evaluate`). The MCP browser runs on the **host**,
so navigate to the dinghy URL **`http://my-family.docker`** (stack must be up). To
authenticate, trigger a magic link in the UI, then read it from the Mailpit inbox at
`http://mail.my-family.docker`. MCP browser output lands in `.playwright-mcp/` at the
repo root.

Use the MCP to *see* a bug (DOM snapshot, console, failing network request) and
confirm a fix; then capture the regression as a headless spec in `fe/e2e/tests`.

## Reminder/time-dependent flows

The worker can expose a test-clock endpoint (`POST /__test/advance-clock` on
`WORKER_METRICS_BIND` :9091) **only under the `test-fixtures` cargo feature** — the
intended hook for deterministic digest/birthday testing. It is feature-gated out of
prod builds; see `crate-worker`.

## Common mistakes

| Symptom | Cause / fix |
|---|---|
| E2E can't reach the app / connection refused | Stack not up — `rdt start`; or you ran it in the `fe` container instead of `playwright`. |
| "browser executable not found" | Ran in the alpine `fe` service; route via `test:e2e`/`exec` to the playwright service. |
| Console-error guard didn't fire | You imported `test` from `@playwright/test` instead of `fixtures/console.fixture`. |
| Flaky / cross-test interference | Expected with `workers: 1` + shared infra; `clearMailpit()` per test, don't enable parallelism. |
| Rate-limit 429 in a sign-in test | Redis buckets not flushed — global-setup runs on CI/`E2E_FLUSH_REDIS`; flush locally if needed. |
