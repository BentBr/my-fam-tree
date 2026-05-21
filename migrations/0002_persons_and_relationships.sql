-- 0002_persons_and_relationships.sql
-- Family members (persons), parent links, and partnerships.

------------------------------------------------------------
-- Enums
------------------------------------------------------------
CREATE TYPE parent_link_kind AS ENUM ('biological', 'legal', 'adoptive', 'step', 'social');
CREATE TYPE partnership_kind AS ENUM ('marriage', 'civil_union', 'partnership');
CREATE TYPE partnership_end_reason AS ENUM ('divorce', 'separation', 'death');

------------------------------------------------------------
-- persons
------------------------------------------------------------
CREATE TABLE persons (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id       UUID NOT NULL REFERENCES families(id) ON DELETE CASCADE,
    given_name      TEXT NOT NULL,
    family_name     TEXT NOT NULL DEFAULT '',
    name_at_birth   TEXT NOT NULL DEFAULT '',
    nickname        TEXT NOT NULL DEFAULT '',
    gender          TEXT NOT NULL DEFAULT '',
    birth_date      DATE,
    birth_place     TEXT NOT NULL DEFAULT '',
    death_date      DATE,
    notes           TEXT NOT NULL DEFAULT '',
    linked_user_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (family_id, linked_user_id)
);
CREATE INDEX persons_family_idx ON persons (family_id);
CREATE INDEX persons_user_idx   ON persons (linked_user_id) WHERE linked_user_id IS NOT NULL;
CREATE TRIGGER persons_touch BEFORE UPDATE ON persons FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

------------------------------------------------------------
-- parent_links
------------------------------------------------------------
CREATE TABLE parent_links (
    child_id   UUID NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    parent_id  UUID NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    kind       parent_link_kind NOT NULL,
    note       TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (child_id, parent_id),
    CHECK (child_id <> parent_id)
);
CREATE INDEX parent_links_parent_idx ON parent_links (parent_id);

------------------------------------------------------------
-- partnerships
------------------------------------------------------------
CREATE TABLE partnerships (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id      UUID NOT NULL REFERENCES families(id) ON DELETE CASCADE,
    partner_a_id   UUID NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    partner_b_id   UUID NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    kind           partnership_kind NOT NULL,
    started_on     DATE,
    ended_on       DATE,
    end_reason     partnership_end_reason,
    note           TEXT NOT NULL DEFAULT '',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (partner_a_id < partner_b_id),
    CHECK (ended_on IS NULL OR started_on IS NULL OR started_on <= ended_on)
);
CREATE INDEX partnerships_family_idx ON partnerships (family_id);
CREATE INDEX partnerships_a_idx ON partnerships (partner_a_id);
CREATE INDEX partnerships_b_idx ON partnerships (partner_b_id);
CREATE TRIGGER partnerships_touch BEFORE UPDATE ON partnerships FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

-- Dedupe identical currently-open partnerships:
-- same canonicalized pair + same kind + currently open (ended_on IS NULL) → only one row allowed.
CREATE UNIQUE INDEX partnerships_unique_open ON partnerships (partner_a_id, partner_b_id, kind)
    WHERE ended_on IS NULL;
