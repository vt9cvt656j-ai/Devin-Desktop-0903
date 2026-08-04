-- Display name and picture, set by the account holder in Settings.
--
-- Names are two columns rather than one "full name": the interface asks for them
-- separately in US order (given name, then family name), and joining them for display is
-- a presentation decision the client makes. Storing the join instead would throw away
-- which half is which, and no query could ever get it back.
--
-- NOT NULL DEFAULT '' rather than nullable: every read path treats "not set" the same as
-- "empty", so a NULL would only add a second way to spell the same state.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS first_name TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS last_name  TEXT NOT NULL DEFAULT '';

-- The picture is a `data:` URL, already resized and re-encoded by the browser before it
-- is sent, so a phone photo arrives as a ~30 KB square rather than 8 MB.
--
-- Held here rather than on disk because the backend runs in a container with no mounted
-- volume of its own — anything written to its filesystem disappears on the next deploy,
-- whereas Postgres has a volume and is covered by the nightly backup. It also keeps the
-- picture inside the same auth check as the rest of the profile: no public file path
-- exists to guess at, and no separate serving route needs its own gate.
--
-- Kept off the `users` SELECT used by the admin list on purpose (see auth.rs): 500 rows
-- each carrying an inline image would make that page many megabytes.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS avatar TEXT;
