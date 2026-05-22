-- 0003_parent_links_cycle_trigger.sql
-- Race-safe backstop for the in-memory cycle check that lives in
-- crates/api/src/routes/parent_links.rs. Two concurrent inserts could each
-- pass the in-memory check on a stale snapshot and then both commit,
-- producing a cycle the app never sees. This trigger reruns the ancestor
-- walk on the candidate row INSIDE the writing transaction so the race
-- window is closed at the DB layer.
--
-- The SERIALIZABLE wrapper in `PgParentLinkRepo::insert` keeps the in-app
-- check correct for the common case; this trigger is the belt-and-braces
-- guarantee.

CREATE OR REPLACE FUNCTION parent_links_no_cycle()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        WITH RECURSIVE ancestors(id) AS (
            SELECT NEW.parent_id
            UNION ALL
            SELECT pl.parent_id FROM parent_links pl
            JOIN ancestors a ON pl.child_id = a.id
        )
        SELECT 1 FROM ancestors WHERE id = NEW.child_id
    ) THEN
        RAISE EXCEPTION 'parent_links cycle: % is already an ancestor of %', NEW.parent_id, NEW.child_id
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER parent_links_no_cycle_trg
    BEFORE INSERT OR UPDATE ON parent_links
    FOR EACH ROW EXECUTE FUNCTION parent_links_no_cycle();
