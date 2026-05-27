---
name: frontend-workflow
description: Use when working in fe/ — building or debugging Vue 3 / Vuetify components, views, Pinia stores, the openapi-fetch API client, TanStack Query hooks, i18n, or Vitest component tests; or when pnpm/lint/typecheck/codegen behave oddly. Covers the container-only tooling rule (never run pnpm/node on the host) and the strict-TS regime. Load before editing anything under fe/src.
---

# Frontend Workflow (Vue 3 + Vuetify + strict TS)

For domain terms see `project-concepts`. For end-to-end / Playwright, load
`playwright-e2e`. For the debugging *method*, **REQUIRED BACKGROUND:**
superpowers:systematic-debugging.

## Container-only tooling (non-negotiable)

There is **no host Node or pnpm**. Every FE command runs in a container via
`scripts/fe-in-container.sh <pnpm-script>` (or the `rdt` wrappers). Never run
`pnpm`, `node`, `npx`, or `vite` directly on the host.

| Need | Command |
|---|---|
| Dev server (Vite :5173 → http://my-family.docker) | `rdt fe` (or `rdt start` for the whole stack) |
| Lint (eslint + typecheck + knip) | `rdt lint` (FE parts) or `scripts/fe-in-container.sh lint` |
| Type-check only | `scripts/fe-in-container.sh typecheck` |
| Component tests (Vitest) | `rdt test` or `scripts/fe-in-container.sh test` |
| Coverage | `scripts/fe-in-container.sh coverage` |
| Regenerate API types after a backend change | `rdt openapi` |

`fe-in-container.sh` routes `test:e2e`/`playwright`/`exec` to the dedicated
`playwright` service; everything else runs `pnpm <script>` in the `fe` service.

## Source layout (`fe/src`)

```
api/           openapi-fetch client, generated schema.d.ts, hooks/, request helpers
components/     common/, layout/, tree/ (d3-based FamilyTree)
views/          page components grouped by route (auth, tree, families, admin, ...)
stores/         Pinia: auth, activeFamily, locale, ui
router/         routes + beforeEach guards (auth hydrate, family reconcile, admin gate)
i18n/           vue-i18n; en.json / de.json
design-system/  Vuetify theme + tokens (TS only)
types/          brand.ts — branded ID types (UserId, FamilyId, ...)
```

`main.ts` installs Pinia, Router, i18n, Vuetify, VueQuery; `App.vue` switches layout
via `route.meta.layout`.

## The API client (the part to get right)

Calls go through the typed `openapi-fetch` client (`src/api/client.ts`) whose
middleware stack injects **`X-Family-Id`** from the activeFamily store, refreshes on
401, translates `problem+json` → `ApiClientError`, and surfaces `meta.warnings` as
toasts. **Do not call the client raw from components.** Use the hooks in
`src/api/hooks/` and the helpers in `src/api/request.ts`:

- `unwrap(call)` — awaits, throws on error, returns inner `data`.
- `expectOk(call)` — for writes whose body you ignore (DELETE/POST).
- `useApiMutation({ mutationFn, success, invalidate, onSuccess })` — wraps
  TanStack `useMutation` with a success toast + query-key invalidation. Errors bubble
  to the central `queryClient` error handler (i18n message from the `code`).

**Generated types:** `src/api/schema.d.ts` is produced by `openapi-typescript` and is
**gitignored — never hand-edit it.** After any backend endpoint change, run
`rdt openapi` (dumps the spec to `fe/openapi.json`, then regenerates the types). An
eslint `no-restricted-imports` rule forbids importing `@/api/schema*` outside
`src/api/` — consume the re-exported types from `src/api/types.ts` instead.

## Strict-TS regime

`tsconfig.json`: `strict` + `noUncheckedIndexedAccess` + `exactOptionalPropertyTypes`
+ `verbatimModuleSyntax` + `noImplicitReturns`/`noFallthroughCasesInSwitch`/
`noPropertyAccessFromIndexSignature`. ESLint: `no-explicit-any` and
`no-non-null-assertion` are **errors**; `consistent-type-imports` enforced (use
`import type`). `knip` fails on dead exports/deps. Branded IDs live in
`src/types/brand.ts` — don't pass bare strings where a branded ID is expected.
Formatting: 4-space indent, `printWidth: 120` (Prettier matches `.editorconfig`).
File size: non-test files soft 300 / hard 500 lines.

## Component tests (Vitest)

`happy-dom` environment. Tests live in `fe/tests/` mirroring `src/`, named
**`*.test.ts`** (never `.spec.ts`). Composables/hooks use
`fe/tests/helpers/hook-wrapper.ts` (`makeHookWrapper`) which mounts a test component
with Pinia + i18n + vue-query. Coverage gate: `fe/src/` ≥ 80% lines. Run with
`rdt test`.

## Common mistakes

| Symptom | Cause / fix |
|---|---|
| `pnpm: command not found` / wrong node_modules | You ran it on the host. Use `scripts/fe-in-container.sh` / `rdt`. |
| eslint error importing `@/api/schema` | Import the re-export from `src/api/types.ts`, not the generated schema. |
| Types stale after backend change | Run `rdt openapi`; don't edit `schema.d.ts` by hand. |
| `any` / `!` rejected by lint | Narrow the type or use a guard; both are hard errors here. |
| Raw `client.GET(...)` in a component | Add/extend a hook in `src/api/hooks/` using `unwrap`/`useApiMutation`. |
