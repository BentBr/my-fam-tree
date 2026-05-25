-- 0005_contacts_and_audit.sql
--
-- Phase 3 — replace the flat persons contact columns with the
-- `person_contacts` table, and introduce `audit_log` for every
-- admin-visible mutation (contacts, persons, parent_links, partnerships,
-- family role changes).
--
-- The 7 flat columns added by `0004_persons_contact_fields.sql` are
-- dropped here outright: the dev DB is reset for Phase 3 (no data
-- migration). Contacts now live in their own table keyed on
-- `person_id`, with a structured JSONB value column and a per-row
-- visibility enum.

ALTER TABLE persons
    DROP COLUMN email,
    DROP COLUMN phone,
    DROP COLUMN street,
    DROP COLUMN house_number,
    DROP COLUMN zip,
    DROP COLUMN city,
    DROP COLUMN country;

CREATE TYPE contact_kind AS ENUM ('email', 'phone', 'address', 'url', 'other');
CREATE TYPE contact_visibility AS ENUM ('family', 'admins_only');

CREATE TABLE person_contacts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id   UUID NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    kind        contact_kind NOT NULL,
    label       TEXT NOT NULL DEFAULT '',
    value       JSONB NOT NULL,
    visibility  contact_visibility NOT NULL DEFAULT 'family',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX person_contacts_person_idx ON person_contacts (person_id);
CREATE TRIGGER person_contacts_touch
    BEFORE UPDATE ON person_contacts
    FOR EACH ROW
    EXECUTE FUNCTION touch_updated_at();

CREATE TABLE audit_log (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id     UUID NOT NULL REFERENCES families(id) ON DELETE CASCADE,
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action        TEXT NOT NULL,
    entity_kind   TEXT NOT NULL,
    entity_id     UUID,
    metadata      JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX audit_log_family_idx ON audit_log (family_id, created_at DESC);
