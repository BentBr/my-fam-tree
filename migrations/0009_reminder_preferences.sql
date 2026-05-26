-- Per-user reminder settings + a digest dispatch log.
--
-- Reminders are NOT per-person subscriptions: each user has a single
-- preferences row mirroring the Upcoming page filter — two event-kind
-- toggles (birthdays, anniversaries), a favourites-only scope toggle, and a
-- single lead_days (0..=21, 0 = on the day). The worker (Phase 4b) projects
-- the events that fall exactly lead_days ahead and sends ONE daily digest.

CREATE TYPE reminder_status AS ENUM ('pending', 'sent', 'failed');

-- One settings row per user. Created lazily on first PUT; the GET endpoint
-- returns built-in defaults when no row exists yet (emails OFF — opt-in).
CREATE TABLE reminder_preferences (
    user_id              UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    emails_enabled       BOOLEAN NOT NULL DEFAULT false,
    remind_birthdays     BOOLEAN NOT NULL DEFAULT true,
    remind_anniversaries BOOLEAN NOT NULL DEFAULT true,
    favourites_only      BOOLEAN NOT NULL DEFAULT false,
    lead_days            INT     NOT NULL DEFAULT 7 CHECK (lead_days >= 0 AND lead_days <= 21),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TRIGGER reminder_preferences_touch BEFORE UPDATE ON reminder_preferences
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

-- One digest per user per local send-date. The UNIQUE constraint is the
-- idempotency key: the worker can re-run a tick within the firing hour and
-- never double-schedule.
CREATE TABLE reminder_digests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    send_date       DATE NOT NULL,
    event_count     INT  NOT NULL,
    status          reminder_status NOT NULL DEFAULT 'pending',
    error           TEXT NOT NULL DEFAULT '',
    attempt_count   INT  NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    dispatched_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, send_date)
);
CREATE INDEX reminder_digests_pending_idx
    ON reminder_digests (next_attempt_at)
    WHERE status = 'pending';
CREATE INDEX reminder_digests_user_idx ON reminder_digests (user_id);
CREATE TRIGGER reminder_digests_touch BEFORE UPDATE ON reminder_digests
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
