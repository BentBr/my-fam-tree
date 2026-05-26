-- 0006_invite_person_id.sql
--
-- Phase D — allow an invite to carry the person row the new member will
-- link to. On accept, the API sets `persons.linked_user_id = new_user.id`
-- inside the same transaction.
--
-- Nullable: an admin may invite without binding to a specific person
-- (e.g. inviting a second admin who isn't represented in the tree).
-- ON DELETE SET NULL so deleting a person doesn't dynamite pending
-- invites — they fall back to family-only membership on accept.

ALTER TABLE family_invites
    ADD COLUMN person_id UUID REFERENCES persons(id) ON DELETE SET NULL;

CREATE INDEX family_invites_person_idx ON family_invites (person_id);
