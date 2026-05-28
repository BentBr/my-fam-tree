-- Phase 5 Task 5 — person photos.
--
-- Photos themselves live in the object store (MinIO in dev, S3 in prod);
-- the DB only carries the storage key. The API resolves the key into a
-- presigned URL on demand (so the browser fetches bytes directly without
-- proxying through the API), and DELETE clears the column + best-effort
-- removes the underlying object.
--
-- Convention: persons.photo_key = 'persons/{person_id}/{nanoid}.jpg'.
-- The nanoid suffix lets us upload a new photo without orphaning the old
-- one for the same person mid-flight; the API deletes the previous key
-- after the new put succeeds.

ALTER TABLE persons ADD COLUMN photo_key TEXT;
