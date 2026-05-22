-- 0004_persons_contact_fields.sql
-- Adds the contact-info columns to `persons` (email + phone + postal address).
--
-- All seven columns are `TEXT NOT NULL DEFAULT ''` so existing rows backfill
-- to empty strings without a separate UPDATE. The API treats empty as "no
-- value"; the FE renders empty as the em-dash placeholder.
--
-- `email` is special: when `linked_user_id` is set, the API layer overrides
-- the column on every write to mirror `users.email`. We deliberately do NOT
-- enforce that here with a trigger — the cross-table lookup belongs in the
-- application layer where it can return a structured `ApiError` if the linked
-- user has vanished. A user changing their own email via PATCH /users/me does
-- NOT yet propagate to linked persons (tracked follow-up).

ALTER TABLE persons
    ADD COLUMN email        TEXT NOT NULL DEFAULT '',
    ADD COLUMN phone        TEXT NOT NULL DEFAULT '',
    ADD COLUMN street       TEXT NOT NULL DEFAULT '',
    ADD COLUMN house_number TEXT NOT NULL DEFAULT '',
    ADD COLUMN zip          TEXT NOT NULL DEFAULT '',
    ADD COLUMN city         TEXT NOT NULL DEFAULT '',
    ADD COLUMN country      TEXT NOT NULL DEFAULT '';
