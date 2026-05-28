-- Phase 5 Task 5 — user avatars.
--
-- Mirror of `persons.photo_key` from migration 0011: the bytes live in the
-- object store, the DB only carries the storage key. The api resolves the
-- key into a presigned URL on demand (so the browser fetches bytes directly
-- without proxying through the api), and DELETE clears the column + best-
-- effort removes the underlying object.
--
-- Convention: users.avatar_key = 'users/{user_id}/{uuid}.jpg'. The random
-- suffix lets us upload a new avatar without orphaning the old one for the
-- same user mid-flight; the api deletes the previous key after the new put
-- succeeds.

ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_key TEXT;
