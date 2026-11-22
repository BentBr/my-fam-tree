---
name: crate-email
description: Use when touching the my-fam-tree-email crate (package `my-fam-tree-email`, crate `my_fam_tree_email`, under crates/email) — adding or changing the EmailSender trait or OutboundEmail, the real SmtpSender (lettre) or FakeEmailSender, the locale-aware Askama text templates (en/de), `render_*` helpers, or the Mailpit integration test. Symptoms: how do I add a new transactional email, why does the Mailpit test skip, where do en/de bodies live, from_dsn schemes.
paths:
  - "crates/email/**"
---

# crate-email (`my-fam-tree-email`)

Outbound transactional email: the `EmailSender` trait + a real SMTP impl + a
fake, and locale-aware bodies — **plain text AND HTML** — rendered from Askama
templates. No Actix/SQLx; standalone in the workspace graph. `api` and `worker`
inject it as `Arc<dyn EmailSender>` (no global state — see `rust-foundations`).
For roles/auth context see `project-concepts`.

`src/lib.rs` re-exports everything (`use my_fam_tree_email::EmailSender;`).

Every email is **multipart/alternative** (plain text + styled HTML). The
`render_*` helpers return a **`RenderedEmail { subject, text, html }`** (not a
tuple); the brand name is the const **`APP_NAME = "My Family Tree"`**, used in
subjects + bodies. The HTML carries the brand sloth logo inline and adapts to
the recipient's light/dark OS setting.

## Module map

| File | Responsibility |
|---|---|
| `src/sender.rs` | `EmailSender` trait (`async fn send(&self, OutboundEmail)`); `OutboundEmail { to_addr, to_name: Option, subject, text_body, html_body: Option }`. |
| `src/smtp.rs` | `SmtpSender` (lettre). `from_dsn(...)`. Real sender → Mailpit. Builds `multipart/alternative { text, multipart/related { html, logo } }` when `html_body` is `Some`; embeds the brand logo (`assets/logo.png`) inline as `cid:logo`. |
| `src/fake.rs` | `FakeEmailSender` — captures sent mail in a `Mutex<Vec<_>>`; `.drain()`/`.len()`/`.is_empty()` for assertions. |
| `src/templates.rs` | Askama structs (one `.txt` + one `.html` per email per locale) + `render_*` helpers returning **`RenderedEmail { subject, text, html }`**; `Chrome` (shared HTML chrome ctx), `APP_NAME`, `ReminderDigestArgs`. |
| `templates/_base.html` | Shared HTML shell (SO short-transactional / Cerberus base) — reset, MSO conditionals, responsive, **`@media (prefers-color-scheme: dark)`**, header logo (`cid:logo`) + footer (`— My Family Tree · Privacy` → `/data-policy`). Children `{% extends %}` it. |
| `templates/_macros.html` | `button` / `paste_link` / `paste_link_box` / `note` macros. |
| `assets/logo.png` | Brand sloth (copy of the FE `apple-touch-icon`), embedded inline. |
| `examples/preview.rs` | `cargo run -p my-fam-tree-email --example preview` → renders all emails (EN/DE) to `target/email-preview/` for browser eyeballing. |
| `src/locale.rs` | `Locale::{En, De}`; `Locale::from_str_or_en(s)` (defaults to En). |
| `src/error.rs` | `EmailError::{Smtp, Config, Build}`. |

`OutboundEmail { to_addr, to_name: Option, subject, text_body, html_body: Option }`
— the whole pipeline (render → `EmailOutboxInsert` → outbox row → worker →
SMTP) carries `html_body` end-to-end. `from_dsn` schemes: `smtp` (plaintext),
`smtp+starttls`, `smtps`; else → `EmailError::Config`. Mailpit DSN `smtp://mailpit:1025`.

## Templates (en/de, text + HTML)

Each email has **four** files: `<name>_{en,de}.txt` (plain text) and
`<name>_{en,de}.html` (HTML). Emails: `magic_link`, `welcome`, `invite`,
`email_change`, `owner_transfer_owner`, `owner_transfer_admin`,
`reminder_digest`.

- **`.txt`** — `escape = "none"`, Askama syntax (`{{ link }}`, `{% if %}`,
  `{% for line in lines %}`).
- **`.html`** — `{% extends "_base.html" %}`, `{% import "_macros.html" as ui %}`
  (import is **top-level**, never inside a block — askama 0.16 rejects it), then
  `{% block preheader %}` + `{% block card %}`. Default (HTML) escaper → every
  `{{ user_value }}` is escaped (XSS-safe). Macro calls need a closing tag:
  `{% call ui::button(link, "Sign in") %}{% endcall %}` (0.16). Authored UI
  literals with HTML entities (`&rsquo;`, `&uuml;`) passed as macro args render
  via `{{ x|safe }}` inside the macro — otherwise they double-escape into a
  visible `&rsquo;`. `Chrome` is a nested struct field on every HTML template so
  `_base.html` can read `chrome.app_url` / `chrome.privacy_url` / etc.

The **subject** is built in Rust (`render_*`), not the template. `render_*` may
take a `name: Option<&str>` to greet a known recipient (magic-link, email-change).

**To add a new email:** create all four files (`foo_{en,de}.{txt,html}`), declare
the text structs (`escape="none"`) **and** the HTML structs (with a `chrome: Chrome`
field), add `pub fn render_foo(locale, app_url, …) -> Result<RenderedEmail, askama::Error>`
that builds the subject + renders both parts, and re-export from `lib.rs`. Then set
`html_body: Some(rendered.html)` at the call site. Templates compile in — a missing
file / undefined var / unbalanced `{% call %}` is a **build** error.

## How to test

- Unit tests (render output, escaping, locale fallback, by-name greeting, fake
  capture) run with no Docker: `cargo test -p my-fam-tree-email`.
- The Mailpit integration test (`tests/mailpit.rs`) sends real SMTP and verifies
  via the Mailpit HTTP API — incl. a test that the HTML part + inline `cid:logo`
  actually arrive. It **skips** unless `EMAIL_DSN` (+ `MAILPIT_API`) are set; run
  it in-network: `./scripts/cargo-in-network.sh test -p my-fam-tree-email --test mailpit`.
- Eyeball the design: `cargo run -p my-fam-tree-email --example preview`, then open
  `target/email-preview/index.html` (toggle OS light/dark for both themes).

## Common mistakes

| Symptom | Fix |
|---|---|
| Added en, forgot de — or added `.txt`, forgot `.html` | Each email is 4 files; `render_*` matches exhaustively on `Locale`. |
| `&rsquo;` shows literally in HTML | It was passed as a macro arg and double-escaped. Use `{{ x|safe }}` in the macro (authored literals only) or a literal Unicode char. |
| `import blocks are not allowed below top level` | Put `{% import %}` at the top of the child template, not inside `{% block %}`. |
| `expected endcall` | askama 0.16 needs `{% call ui::x(...) %}{% endcall %}`. |
| Mailpit test passed but did nothing | It silently returned — `EMAIL_DSN` unset. Use `cargo-in-network.sh`. |
| Subject not localized | Subject is built in the `render_*` fn, not the template. |
| Reaching for a concrete sender in api/worker | Inject `Arc<dyn EmailSender>`; use `FakeEmailSender` in tests. |
| HTML email but `html_body: None` at the call site | `render_*` returns `RenderedEmail`; set `html_body: Some(r.html)`. |
