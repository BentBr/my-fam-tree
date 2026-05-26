-- 0007_owner_transfers.sql
--
-- Phase E - two-token ownership transfer state machine.
--
-- At most ONE pending transfer per family (the partial unique index).
-- A transfer is "pending" while completed_at AND cancelled_at are both
-- NULL. Both tokens must be confirmed before expires_at for the
-- transfer to commit; the `complete` step (BE-side) writes
-- completed_at and atomically swaps the role on family_memberships.

CREATE TABLE family_owner_transfers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id         UUID NOT NULL REFERENCES families(id) ON DELETE CASCADE,
    from_user_id      UUID NOT NULL REFERENCES users(id),
    to_user_id        UUID NOT NULL REFERENCES users(id),
    from_token_hash   BYTEA NOT NULL,
    to_token_hash     BYTEA NOT NULL,
    from_confirmed_at TIMESTAMPTZ,
    to_confirmed_at   TIMESTAMPTZ,
    expires_at        TIMESTAMPTZ NOT NULL,
    completed_at      TIMESTAMPTZ,
    cancelled_at      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX family_owner_transfers_active_idx
    ON family_owner_transfers (family_id)
    WHERE completed_at IS NULL AND cancelled_at IS NULL;

CREATE INDEX family_owner_transfers_from_token_idx
    ON family_owner_transfers (from_token_hash)
    WHERE completed_at IS NULL AND cancelled_at IS NULL;
CREATE INDEX family_owner_transfers_to_token_idx
    ON family_owner_transfers (to_token_hash)
    WHERE completed_at IS NULL AND cancelled_at IS NULL;
