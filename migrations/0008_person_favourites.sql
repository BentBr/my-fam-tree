-- Per-user, per-person favourite marks. Distinct from any family-wide
-- "starred" concept: each row is the *user's own* private pick. Two
-- members of the same family can mark different persons; they never see
-- each other's favourites. The composite primary key both enforces
-- idempotency (re-favouriting is a no-op) and gives us the natural
-- lookup index by user. The secondary index supports the future "who
-- has this person favourited" projection without forcing a sort.

CREATE TABLE person_favourites (
    user_id    UUID NOT NULL REFERENCES users(id)   ON DELETE CASCADE,
    person_id  UUID NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, person_id)
);
CREATE INDEX person_favourites_user_idx   ON person_favourites (user_id);
CREATE INDEX person_favourites_person_idx ON person_favourites (person_id);
