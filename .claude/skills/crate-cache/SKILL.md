---
name: crate-cache
description: Use when working in the cache crate (package my-family-cache, crate my_family_cache) — the Redis access layer providing the connection pool, rate limiter, and reminder job queue. Triggers — adding/changing Redis-backed pooling, rate limiting, or the digest queue; wiring these into api or reminder-worker; deciding key namespacing. Keywords — RedisPool, RedisRateLimiter, RedisReminderQueue, rate limit, leader lock, job queue, key prefix, deadpool-redis.
---

# crate-cache — Redis access layer

## Overview

`my-family-cache` is the standalone Redis layer (no domain dependency). It wraps
`deadpool-redis` + the `redis` crate and exposes three things, each behind a trait so
`AppState`/`WorkerState` can inject an `Arc<dyn …>` (and fakes in tests). Consumed by
`crate-api` (rate limiter) and `crate-reminder-worker` (job queue + the leader lease).
For workspace conventions see `rust-foundations`; for the domain, `project-concepts`.

`src/lib.rs` re-exports: `RedisPool`, `RateLimiter`, `RateLimitDecision`,
`RedisRateLimiter`, `ReminderJob`, `ReminderJobQueue`, `RedisReminderQueue`, `CacheError`.

## Module map

| Module | Contents |
|---|---|
| `pool.rs` | `RedisPool` — `build(url, max_size, key_prefix)`, `prefix()`, `inner()`, `ping()` |
| `rate_limit.rs` | `RateLimiter` trait + `RedisRateLimiter` + `RateLimitDecision` |
| `job_queue.rs` | `ReminderJobQueue` trait + `RedisReminderQueue` + `ReminderJob { digest_id }` |
| `error.rs` | `CacheError` — `Pool` / `Redis` / `Config(String)` |

## Key types & patterns

- **Key prefix discipline.** `RedisPool` carries a `key_prefix` and *every* key is
  built as `format!("{}…", self.prefix())`: `…rate:{key}`, `…queue:reminder-digest`.
  Prod prefix is `my-family:` (api/worker config default, `REDIS_KEY_PREFIX`); api
  tests use `t:`; this crate's own tests use a unique per-run prefix. Any new key MUST
  go through `prefix()` — never hardcode a bare key.
- **`RedisRateLimiter`** — sliding-window-log via a sorted set, one pipelined
  round-trip (`ZREMRANGEBYSCORE` + `ZADD` + `ZCARD` + `EXPIRE`), decision client-side.
  Deliberately *not* `MULTI/EXEC`. Backs magic-link per-email / per-IP limits.
- **`RedisReminderQueue`** — `LPUSH` + non-blocking `RPOP` ⇒ FIFO. `try_pop` returns
  `Ok(None)` when empty (dispatchers poll on a sleep, no `BRPOP`). `reminder_digests`
  is the source of truth, so a dropped queue is recoverable.
- **Leader lock lives in `crate-reminder-worker`** (`src/leader.rs`, `SET … NX PX`),
  *not* here — it merely borrows this crate's `RedisPool` + `prefix()`. Do not add it
  to cache.

## How to test

Integration tests in `tests/{ping,rate_limit,job_queue}.rs` require a running Redis and
**skip silently when `REDIS_URL` is unset** (not testcontainers — unlike the api harness
in `rust-foundations`). Each uses a unique prefix for parallel isolation. Run inside the
compose network so Redis resolves: `./scripts/cargo-in-network.sh test -p my-family-cache`,
or with `REDIS_URL` exported: `cargo test -p my-family-cache`.

## Common mistakes

- Hardcoding a Redis key without `pool.prefix()` — breaks namespacing and test isolation.
- Expecting `try_pop` to block — it does not; the caller polls.
- Looking for the leader lock here — it's in `crate-reminder-worker`.
- Assuming `tests/` boot Redis — they no-op without `REDIS_URL`; use `cargo-in-network.sh`.
