-- Tree connector style as an account preference.
--
-- How the family tree draws parent → child lines: 'curved' (smooth Bezier,
-- the look every existing account has seen so far) or 'angular' (right-angled
-- elbows). Stored on the account, like `users.locale`, so the choice follows
-- the user across devices; the frontend additionally mirrors it into
-- localStorage for signed-out visits.

CREATE TYPE user_edge_style AS ENUM ('curved', 'angular');

ALTER TABLE users ADD COLUMN edge_style user_edge_style NOT NULL DEFAULT 'curved';
