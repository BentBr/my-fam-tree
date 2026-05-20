-- 0001_auth_and_families.sql
-- Authentication, users, families, memberships, invites.

CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "citext";

------------------------------------------------------------
-- Enums
------------------------------------------------------------
CREATE TYPE family_role AS ENUM ('user', 'admin', 'owner');
CREATE TYPE magic_link_purpose AS ENUM ('login', 'invite_accept', 'email_change');
CREATE TYPE user_locale AS ENUM ('en', 'de');

------------------------------------------------------------
-- users
------------------------------------------------------------
CREATE TABLE users (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email              CITEXT UNIQUE NOT NULL,
    display_name       TEXT NOT NULL DEFAULT '',
    locale             user_locale NOT NULL DEFAULT 'en',
    timezone           TEXT NOT NULL DEFAULT 'Europe/Berlin',
    email_verified_at  TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX users_email_idx ON users (email);

------------------------------------------------------------
-- magic_link_tokens
------------------------------------------------------------
CREATE TABLE magic_link_tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID REFERENCES users(id) ON DELETE CASCADE,
    token_hash   BYTEA NOT NULL UNIQUE,
    purpose      magic_link_purpose NOT NULL,
    email        CITEXT NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX magic_link_tokens_expires_idx ON magic_link_tokens (expires_at) WHERE consumed_at IS NULL;
CREATE INDEX magic_link_tokens_email_idx ON magic_link_tokens (email) WHERE consumed_at IS NULL;

------------------------------------------------------------
-- refresh_tokens
------------------------------------------------------------
CREATE TABLE refresh_tokens (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash          BYTEA NOT NULL UNIQUE,
    device_label        TEXT,
    user_agent          TEXT,
    ip                  INET,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ NOT NULL,
    absolute_expires_at TIMESTAMPTZ NOT NULL,
    revoked_at          TIMESTAMPTZ
);
CREATE INDEX refresh_tokens_user_idx ON refresh_tokens (user_id) WHERE revoked_at IS NULL;

------------------------------------------------------------
-- families
------------------------------------------------------------
CREATE TABLE families (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    created_by  UUID NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

------------------------------------------------------------
-- family_memberships
------------------------------------------------------------
CREATE TABLE family_memberships (
    family_id  UUID NOT NULL REFERENCES families(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role       family_role NOT NULL,
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (family_id, user_id)
);
CREATE INDEX family_memberships_user_idx ON family_memberships (user_id);

-- Exactly one owner per family (enforced by partial unique index).
CREATE UNIQUE INDEX family_memberships_one_owner ON family_memberships (family_id)
    WHERE role = 'owner';

------------------------------------------------------------
-- family_invites
------------------------------------------------------------
CREATE TABLE family_invites (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id     UUID NOT NULL REFERENCES families(id) ON DELETE CASCADE,
    email         CITEXT NOT NULL,
    invited_role  family_role NOT NULL,
    invited_by    UUID NOT NULL REFERENCES users(id),
    token_hash    BYTEA NOT NULL UNIQUE,
    expires_at    TIMESTAMPTZ NOT NULL,
    accepted_at   TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX family_invites_email_idx ON family_invites (email) WHERE accepted_at IS NULL;
CREATE INDEX family_invites_family_idx ON family_invites (family_id) WHERE accepted_at IS NULL;

------------------------------------------------------------
-- updated_at triggers
------------------------------------------------------------
CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END $$ LANGUAGE plpgsql;

CREATE TRIGGER users_touch     BEFORE UPDATE ON users     FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER families_touch  BEFORE UPDATE ON families  FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
