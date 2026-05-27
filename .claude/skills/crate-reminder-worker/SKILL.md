---
name: crate-reminder-worker
description: Use when working in the reminder-worker crate (package my-family-reminder-worker, crate my_family_reminder_worker) — the leader-locked scheduler + dispatcher pool that sends daily digest emails. Triggers when editing the leader loop, ticker, dispatcher, backoff, digest projection, the Clock abstraction, or the test-fixtures advance-clock endpoint. Keywords reminder worker, digest, leader lock, ticker, dispatcher, Clock, FixedClock, SystemClock, test-fixtures, advance-clock, backoff, WorkerState.
---

# crate-reminder-worker

Load `project-concepts` for the domain and `rust-foundations` for the strict
lint gate, errors, IDs, and SQLx — not repeated here.

## Overview

A long-running binary that emails each user a daily **digest** of upcoming
birthdays/anniversaries at their local **06:00**. One Redis-elected leader runs
the scheduling tick; every replica runs a pool of dispatchers that render and
send. It is a thin wrapper: orchestration lives in the lib so integration tests
(`tests/digest_flow.rs`) drive the ticker/dispatcher directly.

## Module map

| Module | Role |
|---|---|
| `main.rs` | Wires prod collaborators into `WorkerState`, spawns dispatchers, runs the leader loop. Inline tracing init (json vs pretty). |
| `state.rs` | `WorkerState` DI container: `clock` + repos + `queue` + `email`, all `Arc<dyn …>`; plus `web_public_url`, `max_retries`, `retry_min/max_seconds`. |
| `config.rs` | `Config::from_env()` via `envy` — all `WORKER_*` + db/redis/email vars. |
| `clock.rs` | `Clock` trait; `SystemClock` (prod) vs `FixedClock` (settable atomic). |
| `leader.rs` | Redis-lease single-leader election. |
| `ticker.rs` | `run_tick(&state)` — schedules digests, returns count. |
| `dispatcher.rs` | `run_dispatcher(state)` loop + `handle(&state, &job)`. |
| `backoff.rs` | `next_attempt()` exponential backoff + 25% jitter. |
| `digest.rs` | `events_for_user_on` projection (reuses domain `build_upcoming`) + localized `render_line`. |
| `test_clock_http.rs` | test-fixtures-only HTTP listener. |

## Architecture / control flow

`main` builds `WorkerState`, spawns `DISPATCHER_POOL = 4` `run_dispatcher`
tasks, then loops: `leader.acquire_blocking()` (polls every 30s until it wins
`SET key NX PX ttl`) → inner loop calls `leader.refresh()` (Lua compare-and-
PEXPIRE; `false` ⇒ lost lease ⇒ re-acquire) and `ticker::run_tick` once per
`tick` interval, sleeping `refresh` between checks.

- **Only the leader ticks**; dispatchers run on **all** replicas.
- `run_tick`: for each `prefs.enabled_user_ids()`, if the user's local hour is
  06, project events on `today + lead_days`, then `digests.ensure_pending`
  (idempotent) and, if freshly inserted, `queue.push`. Safe to re-run hourly.
- `handle`: load digest+user+prefs, **re-project at send time** (so edits land),
  render localized text, `email.send`. SMTP failure is not an error — it marks
  the row failed/retry via `backoff::next_attempt` and re-queues until
  `max_retries`.

## The test-fixtures feature (clock control)

`FixedClock` (an `Arc<AtomicI64>` of micros) replaces `SystemClock` only under
`--features test-fixtures`, so tests/E2E fast-forward time. The feature also
compiles `test_clock_http`: an Actix listener on `WORKER_METRICS_BIND` exposing
`POST /__test/advance-clock` `{ "to": <RFC3339> }` that sets the clock and runs
a tick immediately. It runs on a **dedicated `std::thread` with its own actix
`System`** because `HttpServer`'s future is `!Send`. **NEVER** built into the
prod image (the Dockerfile's release build is feature-free); the FE E2E advances
the clock through this endpoint.

## Run & debug

- Run: `cargo run -p my-family-reminder-worker --bin reminder-worker` (or `rdt worker`).
- With the test clock: add `--features test-fixtures`.
- Logs: `docker compose logs -f reminder-worker`.
- Env (compose defaults): `WORKER_TICK_INTERVAL_SECONDS=300`,
  `WORKER_LEADER_LEASE_SECONDS=60`, `WORKER_LEADER_REFRESH_SECONDS=20`,
  `WORKER_MAX_RETRIES=5`, `WORKER_RETRY_BACKOFF_MIN_SECONDS=60`,
  `WORKER_RETRY_BACKOFF_MAX_SECONDS=43200`, `WORKER_METRICS_BIND=0.0.0.0:9091`,
  plus `WEB_PUBLIC_URL`.

## Common mistakes

- Putting projection logic here — `digest.rs` reuses domain `build_upcoming` so
  the digest never drifts from the `/upcoming` route. Don't fork it.
- Treating SMTP errors as fatal: `handle` records + retries; only repo/queue/
  template errors propagate.
- Adding `tokio::spawn` for the test HTTP server — it's `!Send`; keep the
  dedicated-thread + actix `System` pattern, feature-gated.
- Forgetting tick idempotency: rely on `ensure_pending`'s `inserted` flag before
  pushing, or you double-send within the 06:00 hour.
