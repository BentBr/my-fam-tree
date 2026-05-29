---
name: crate-api
description: Use when adding or changing an HTTP endpoint, route, or handler in the my-fam-tree API; touching AppState, build_app, AuthMiddleware, the X-Family-Id flow, the ApiError/ErrorCode/RFC 7807 error model, the ApiResponse/response_body! success envelope, or the utoipa::path/openapi aggregation. Triggers: "add an endpoint", "new route", "ApiError mapping", "openapi", Actix handler work.
---

# crate-api (`my-fam-tree-api`, `my_fam_tree_api`)

The Actix-web HTTP server: routes, DI container, middleware, the locked-down
error/response contract, and the aggregated OpenAPI doc. For the domain model and
the auth/envelope *concepts*, see `project-concepts`; for deny-lints, SQLx, and the
testcontainers harness, see `rust-foundations`. This skill goes deeper on api-crate
mechanics.

## Module map

| Path | What it owns |
|---|---|
| `src/lib.rs` | `build_app(state, openapi)` — middleware stack + `routes::api_scope()` + optional Swagger UI. Re-exports `Config`/`AppEnv`/`LogFormat`/`AppState`/`ApiError`/`ApiResponse`/etc. |
| `src/state.rs` | `AppState` — the DI container (see below). |
| `src/config.rs` | `Config` (figment from env), `AppEnv`, `LogFormat`; cross-field `validate()` (e.g. `JWT_PRIVATE_KEY_ID` must appear in `JWT_PUBLIC_KEYS`). |
| `src/error.rs` | `ApiError`, `ErrorCode`, `ApiErrorBody`, `FieldViolation`, `ApiResult<T>`. |
| `src/response.rs` | `ApiResponse<T>`, `ResponseMeta`, `Pagination`, `Warning`, `response_body!`. **Only file allowed to build `HttpResponse`.** |
| `src/routes/mod.rs` | `api_scope()` — `/api/v1` public + auth-wrapped sub-scope. One file per resource under `routes/`. |
| `src/auth/` | `JwtIssuer`/`JwtKeyset` (Ed25519 PEM, `kid` rotation), `JwtClaims`/`FamilyClaim`, `AuthMiddleware`, `user_claims*`, `require_role`, token helpers. |
| `src/middleware/` | `PanicCatcher`, `RequestId`. |
| `src/services/` | `auth_service`, `audit`, `relationships_tree`, `upcoming` — orchestration between repos and thin handlers. |
| `src/validation/` | request validation (`looks_like_email`, `email_invalid`, `string_too_long`, …) incl. `relationships`. |
| `src/openapi_doc.rs` | `ApiDoc` aggregating every `utoipa::path` + schema. |

## AppState (the DI container)

`#[derive(Clone)]` struct of `Arc<dyn …Repo>` + `email` + `rate_limiter` + `redis`
+ `Arc<JwtIssuer>` + `Arc<Config>` (`cfg`). Handed to handlers as
`web::Data<AppState>`. Production wiring lives in `bin/api.rs`; tests substitute
fakes. `Debug` is intentionally opaque (`finish_non_exhaustive`) so key material /
pools never leak into logs.

## The handler pattern (most important thing)

Every handler is attribute-stacked and returns the envelope-or-error result:

```rust
#[utoipa::path(get, path = "/api/v1/reminder-preferences",
    responses((status = 200, body = ReminderPrefsResponseBody), (status = 401)),
    security(("cookie_access" = [])), tag = "reminders")]
#[allow(clippy::future_not_send)]   // !Send future; lint is irrelevant here
#[allow(unreachable_pub)]           // actix's #[get]/#[post] wrap the fn
#[get("/reminder-preferences")]
pub async fn get_prefs(state: web::Data<AppState>, req: HttpRequest)
    -> Result<ApiResponse<ReminderPrefsView>, ApiError>
{
    let claims = crate::auth::user_claims(&req)?;            // who
    let prefs = state.reminder_prefs.get(claims.user_id).await.map_err(internal)?;
    Ok(ApiResponse::ok(prefs.into()))
}
```

- Body input: `body: web::Json<FooReq>`. Always return `Result<ApiResponse<T>, ApiError>`.
- Identity: `user_claims(&req)?` (any member) or `user_claims_with_family(&req)?`
  → `(UserClaims, ActiveFamily)` when an active family is mandatory (surfaces the
  missing-`X-Family-Id` 422). Then `require_role(&active, Role::Admin)?` to gate.
- Repo errors → `ApiError::Internal(anyhow::anyhow!(e.to_string()))` (handlers
  often define a local `fn internal(e)` helper). Map *expected* repo errors to
  specific `ApiError` variants instead (e.g. `MagicLinkInvalid`).
- **Cookie-setting handlers are the exception**: `/auth/consume`, `/auth/refresh`,
  `/auth/logout` return `Result<HttpResponse, ApiError>` so they can `add_cookie`.

## Auth & middleware

`build_app` wraps (logical order) `CORS → RequestId → TracingLogger → Logger →
PanicCatcher → handler` (actix wraps in reverse, so registration is reversed).
`api_scope()` mounts public routes, then a nested `web::scope("")` wrapped by
`AuthMiddleware::required()`. The middleware verifies the `access` JWT cookie,
parses `X-Family-Id`, cross-references it against the `families[]` claim, and inserts
`UserClaims` into request extensions. A header that matches no membership is
*silently dropped* (no active family) — handlers needing one call
`user_claims_with_family`. `PanicCatcher` and the middleware return
`Err(actix_web::Error::from(ApiError))` (NOT `into_parts/from_parts`, which panics).

## Error & response model specifics

`ApiResponse<T> = { data, meta? }`; `meta` carries `pagination` / `request_id` /
`warnings`. utoipa 5 has **no generic `ToSchema`**, so every endpoint declares a
named wrapper via `response_body!(pub FooResponseBody, Foo)` used as
`body = FooResponseBody` — runtime handlers still return `ApiResponse<T>`; wrappers
are schema-only. DELETE returns `{ "data": null }` via `NullResponseBody`.

`ApiError` → RFC 7807 `application/problem+json` with stable `ErrorCode` (`.slug()`
e.g. `auth.unauthenticated`, `family.not_member`). Status mapping (`ErrorCode::http_status`):
401 auth/token; 403 not-member / insufficient-role / not-editable; 404 not-found;
409 cycle / duplicate / stale / email-taken / transfer-pending; 410 invite-expired;
422 `Validation(Vec<FieldViolation>)`; 429 rate-limited (adds `Retry-After`);
502 upstream; 500 internal (detail sanitized, full chain logged via tracing).

## Adding an endpoint (checklist)

1. Write the handler in `routes/<resource>.rs`: `#[utoipa::path]` +
   `#[allow(clippy::future_not_send)]` + `#[allow(unreachable_pub)]` + actix method
   macro; return `Result<ApiResponse<T>, ApiError>`.
2. Declare a `response_body!(pub FooResponseBody, FooView)` wrapper.
3. Register the service in `routes/mod.rs` — **public** routes directly on the
   `/api/v1` scope, **auth-required** routes inside the empty-path sub-scope.
4. In `src/openapi_doc.rs`: add the fn to `paths(...)` and the wrapper + any new
   request/payload structs to `components(schemas(...))`.
5. `rdt openapi` — regenerates `fe/openapi.json` + FE TS types; **commit the updated
   `fe/openapi.json`** (CI `openapi-check` fails on drift). Full pipeline: `project-concepts`.
6. Add an integration test in `crates/api/tests/<resource>_flow.rs`.
7. `rdt lint && rdt test`.

## Gotchas / common mistakes

| Symptom | Cause / fix |
|---|---|
| New route 404s in tests | Two `web::scope("/api/v1")` shadow each other. Use the single scope + nested empty sub-scope for middleware grouping. |
| `ToSchema` derive fails on `ApiResponse<T>` | Generic not supported — add a `response_body!` wrapper. |
| Endpoint absent from `fe/openapi.json` | Forgot `paths(...)`/`schemas(...)` in `openapi_doc.rs`, or didn't run `rdt openapi`. CI `openapi-check` diffs it. |
| `HttpResponse` rejected at review | Only `response.rs` (+ the 3 cookie handlers) may build raw responses. |
| Middleware/panic recovery panics | Return `Err(Error::from(ApiError))`, never `into_parts/from_parts`. |
| Handler bypasses role check | Gate with `require_role(&active, …)?` after `user_claims_with_family`. |

## How to test

The integration suite uses the testcontainers harness in
`crates/api/tests/common/mod.rs`: `ephemeral_stack()` boots throwaway Postgres +
Redis, migrates, builds `AppState` with `FakeEmailSender`. Build the app with
`my_fam_tree_api::build_app(stack.state.clone(), None)`; helpers: `sign_in`,
`create_family`, `try_call` (use instead of `test::call_service` when
`AuthMiddleware` may return `Err`), `extract_token_from_link`. Run one suite:
`cargo test -p my-fam-tree-api --test auth_flow -- --nocapture` (needs a Docker daemon).
