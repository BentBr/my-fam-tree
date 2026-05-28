-- Phase 5 Task 13 — durable transactional-email outbox.
--
-- The API used to call SmtpSender.send() from the request handler, so a slow
-- or dead SMTP server blocked the request thread AND a Redis flush could
-- lose any in-flight email entirely (Redis is in-memory). Producers now
-- INSERT into email_outbox inside the same Postgres transaction as the
-- user-visible side effect (magic-link issued, invite created, transfer
-- begun, etc.) and the worker drains the table via the existing
-- EmailSender. Postgres is durable, so a Redis flush no longer loses mail
-- and SMTP slowness no longer ties up API request threads.

-- Idempotent: Postgres has no `CREATE TYPE IF NOT EXISTS`. The DO-block + EXCEPTION
-- handler turns a duplicate-object error into a no-op so the migration can replay
-- over a half-applied state (e.g. manual hot-fix before the recompiled migrator binary).
DO $$ BEGIN
    CREATE TYPE email_outbox_status AS ENUM ('pending', 'sent', 'failed_permanent');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS email_outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Logical email kind ("magic_link", "invite", "owner_transfer_from",
    -- "owner_transfer_to", "email_change", …). TEXT (not enum) so new kinds
    -- ship without a migration; the worker treats it as an opaque label
    -- (used only for tracing / metrics).
    kind            TEXT NOT NULL,
    to_addr         CITEXT NOT NULL,
    subject         TEXT NOT NULL,
    -- Bodies are rendered at INSERT time, so the worker just SMTPs them —
    -- no template/locale machinery in the worker path. html_body is NULL
    -- when the kind is a plain-text email (today's only shape).
    text_body       TEXT NOT NULL,
    html_body       TEXT,
    status          email_outbox_status NOT NULL DEFAULT 'pending',
    attempts        INT  NOT NULL DEFAULT 0,
    -- The worker claims rows whose next_attempt_at <= now() (and status =
    -- pending). Retries push this forward with exponential backoff.
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error      TEXT,
    sent_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Hot path: "claim the next due pending email". Partial index keeps it tight.
CREATE INDEX IF NOT EXISTS email_outbox_due_idx
    ON email_outbox (next_attempt_at)
    WHERE status = 'pending';

-- Cold queries (debug + janitor): full-table scans by status/created_at are
-- rare enough that we don't add a separate index.
