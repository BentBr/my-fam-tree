-- Composite index for the admin family-overview "latest 3 persons"
-- query and any future paginate-by-created-at flows.
--
-- The base index `persons_family_idx ON persons (family_id)`
-- (`0002_persons_and_relationships.sql`) is enough for the
-- `WHERE family_id = $1` lookups, but the admin overview adds
-- `ORDER BY created_at DESC LIMIT 3` on top, which forces a sort over
-- every family row. For our seeded family that's harmless; for a
-- realistic family of a few hundred persons it would still scale
-- fine; for a multi-thousand-row family it would not. A composite
-- (family_id, created_at DESC, id DESC) lets Postgres satisfy both
-- the filter and the order from the index directly — no sort step.
--
-- Including `id` as the third key gives the order a deterministic
-- tiebreak (two persons created in the same microsecond would
-- otherwise reshuffle across queries) so the FE's pagination /
-- caching never lands on a contradictory pair across reloads.
CREATE INDEX IF NOT EXISTS persons_family_created_at_idx
    ON persons (family_id, created_at DESC, id DESC);
