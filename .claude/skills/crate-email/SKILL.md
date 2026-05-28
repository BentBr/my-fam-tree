---
name: crate-email
description: Use when touching the my-family-email crate (package `my-family-email`, crate `my_family_email`, under crates/email) — adding or changing the EmailSender trait or OutboundEmail, the real SmtpSender (lettre) or FakeEmailSender, the locale-aware Askama text templates (en/de), `render_*` helpers, or the Mailpit integration test. Symptoms: how do I add a new transactional email, why does the Mailpit test skip, where do en/de bodies live, from_dsn schemes.
---

# crate-email (`my-family-email`)

Outbound transactional email: the `EmailSender` trait + a real SMTP impl + a
fake, and locale-aware plain-text bodies rendered from Askama templates. No
Actix/SQLx; standalone in the workspace graph. `api` and `worker`
inject it as `Arc<dyn EmailSender>` (no global state — see `rust-foundations`).
For roles/auth context see `project-concepts`.

`src/lib.rs` re-exports everything (`use my_family_email::EmailSender;`).

## Module map

| File | Responsibility |
|---|---|
| `src/sender.rs` | `EmailSender` trait (`async fn send(&self, OutboundEmail)`); `OutboundEmail { to_addr, to_name: Option, subject, text_body, html_body: Option }`. |
| `src/smtp.rs` | `SmtpSender` (lettre). `from_dsn(dsn, from_name, from_address, reply_to: Option, timeout_secs)`. Real dev/prod sender → Mailpit. |
| `src/fake.rs` | `FakeEmailSender` — captures sent mail in a `Mutex<Vec<_>>`; `.drain()`/`.len()`/`.is_empty()` for assertions. |
| `src/templates.rs` | Askama template structs + `render_*` helpers returning `(subject, body)`; `ReminderDigestArgs`. |
| `src/locale.rs` | `Locale::{En, De}`; `Locale::from_str_or_en(s)` (defaults to En). |
| `src/error.rs` | `EmailError::{Smtp, Config, Build}`. |

`from_dsn` schemes: `smtp` (plaintext, no TLS), `smtp+starttls`, `smtps`;
anything else → `EmailError::Config`. Mailpit dev DSN is `smtp://mailpit:1025`.

## Templates (en/de)

`templates/` holds a `<name>_en.txt` / `<name>_de.txt` pair per email:
`magic_link`, `invite`, `email_change`, `owner_transfer_admin`,
`owner_transfer_owner`, `reminder_digest`. They're plain text with
`escape = "none"` (URLs/names are trusted) and use Askama syntax —
`{{ link }}`, `{% if %}`, `{% for line in lines %}`. The **subject** is built in
Rust (often `format!`), not in the template; only the **body** is rendered.

**To add a new email:** create both `templates/foo_en.txt` and `foo_de.txt`,
declare a `#[derive(Template)] #[template(path = "foo_en.txt", escape = "none")]`
struct (one per locale) with the interpolated fields, add a
`pub fn render_foo(locale: Locale, ...) -> Result<(String, String), askama::Error>`
that matches on `Locale` and calls `.render()?`, and re-export it from `lib.rs`.
Templates compile into the binary, so a missing file or undefined var is a
**build** error, not runtime.

## How to test

- Unit tests (render output, locale fallback, fake capture) run with no Docker:
  `cargo test -p my-family-email`.
- The Mailpit integration test (`tests/mailpit.rs`) sends real SMTP and verifies
  via the Mailpit HTTP API. It **skips** unless `EMAIL_DSN` (+ `MAILPIT_API`)
  are set. Run it in the compose network, which injects both:
  `./scripts/cargo-in-network.sh test -p my-family-email --test mailpit`.

## Common mistakes

| Symptom | Fix |
|---|---|
| Added an en template, forgot de | Always add both `_en` + `_de`; render matches exhaustively on `Locale`. |
| Mailpit test passed but did nothing | It silently returned — `EMAIL_DSN` unset. Use `cargo-in-network.sh`. |
| Subject not localized | Subject is built in the `render_*` fn, not the template. |
| New `.txt` not found at build | Path is relative to `templates/`; rebuild — templates are compile-time. |
| Reaching for a concrete sender in api/worker | Inject `Arc<dyn EmailSender>`; use `FakeEmailSender` in tests. |
