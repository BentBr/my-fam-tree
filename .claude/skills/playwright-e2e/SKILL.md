---
name: playwright-e2e
description: Use when writing, running, or debugging Playwright end-to-end tests in fe/e2e, or when reproducing/inspecting a frontend bug in a real browser. Covers the dedicated playwright compose service, the magic-link/Mailpit + console-error + link-rewrite fixtures, global Redis-flush/DB-truncate setup, data-testid selectors, traces/reports, and live browser debugging via the Playwright MCP. Load for E2E flakiness, "test can't reach the app", or full-stack flow tests.
paths:
  - "fe/e2e/**"
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

Inside the container `E2E_BASE_URL=http://my-fam-tree.docker:5173` and
`MAILPIT_URL=http://mail.my-fam-tree.docker:8025` (set in `compose.yaml`).

## Config & isolation (`fe/e2e/playwright.config.ts`)

- `baseURL = E2E_BASE_URL ?? http://my-fam-tree.docker`; project `e2e` = Desktop Chromium;
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
| `fixtures/email-links.fixture.ts` | `rewriteEmailLink(link)` — maps the email's `http://my-fam-tree.docker` URL to the in-container `E2E_BASE_URL` |
| `page-objects/login.page.ts` | `LoginPage` (locators by `data-testid`: `login-card`, `sign-in-email`, `sign-in-submit`, `sign-in-sent`, `login-error`) |

**Selectors:** use `data-testid` (via `getByTestId`), not CSS/text. If a component
lacks a testid you need, add one to the component.

## Shared page-objects — use these, NEVER inline the flow

The auth / family / seeding flows are factored into `page-objects/`. **Always
reuse them; do not re-implement a sign-in, an invite round-trip, a person POST,
or an email-link extraction inline.** Copy-pasting these blocks is exactly the
churn we keep deleting — if a flow recurs and no helper fits, add one to the
right page-object rather than inlining (see `using-superpowers` DRY rule).

| `page-objects/` | Helpers |
|---|---|
| `session.ts` | `signIn(page, email)` (full magic-link round-trip → lands post-auth); `signedInContext(browser, email)` → `{ context, page }` (fresh context + newPage + signIn — for multi-user tests, caller `context.close()`s); `createFamily(page, name)` → familyId; `consumeLinkFromEmail(email)` → rewritten `/auth/consume` link; `AUTH_EMAIL_SUBJECT` (the sign-in **and** welcome subject matcher, EN+DE). |
| `seed.ts` | `seedPerson(request, familyId, data)` → id; `seedParentLink(request, familyId, childId, parentId, kind?)`; `seedPartnership(request, familyId, aId, bId, opts?)`; `seedContact(request, familyId, personId, data)`; `bulkSeedFamily(request, familyId)` + `GENERATIONS/PER_GEN/TOTAL`. All POST via the page's `request` (cookie jar) — no UI clicks. |
| `invites.ts` | `inviteAndAccept(ownerPage, inviteePage, familyId, email, role)` (owner invites + invitee accepts via the emailed link); `acceptInviteFromEmail(page, email)`; `INVITE_EMAIL_SUBJECT`. |

**Magic-link sign-in:** just `await signIn(page, email)`. Under the hood it's
`clearMailpit()` → fill `sign-in-email` + submit → `waitForEmail` on
`AUTH_EMAIL_SUBJECT` → `consumeLinkFromEmail` (extract `/auth/consume?token=…`,
`rewriteEmailLink`) → `page.goto` → assert post-auth redirect. Subjects carry the
brand **"My Family Tree"** (e.g. `Welcome to My Family Tree`, `Sign in to My
Family Tree`); a brand-new account gets the welcome variant, returning users the
sign-in one — `AUTH_EMAIL_SUBJECT`/`consumeLinkFromEmail` accept both. Tests are
`*.test.ts` (never `.spec.ts`).

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
so navigate to the dinghy URL **`http://my-fam-tree.docker`** (stack must be up). To
authenticate, trigger a magic link in the UI, then read it from the Mailpit inbox at
`http://mail.my-fam-tree.docker`. MCP browser output lands in `.playwright-mcp/` at the
repo root (gitignored). To preview a **local** HTML file (e.g. `target/email-preview/`),
the MCP browser blocks the `file:` scheme — serve the dir over HTTP first
(`python3 -m http.server`) and navigate to `http://localhost:<port>/…`.

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
| Consume sticks on `/auth/consume` ("link may have expired"), passes locally | The single-use token's consume fired **twice** (route double-mount) and the 2nd 401'd. `ConsumeView` dedupes via a module-level in-memory set + sessionStorage + a benign-when-authenticated catch. Note: at the in-network `:5173` origin sessionStorage can be partitioned/blocked, so storage-only dedup is NOT enough — the in-memory guard is the real fix. |
| Repros only in CI, not locally | CI uses a slow runner + cold-started debug backend (`big_family` seed alone can take ~15 s); local fast machines hide timing races. Run the **full** suite (or rebuild api/worker) to surface them. Note `[profile.dev.package."*"] opt-level = 2` keeps debug deps fast enough to avoid spurious cold-start timeouts. |
| `reminders`/digest test 404s on `advance-clock` after a local rebuild | You rebuilt the worker with `docker compose build worker` (prod target). The `/__test/advance-clock` hook needs the `test-fixtures` feature; CI builds it in, a plain prod rebuild does not. |
