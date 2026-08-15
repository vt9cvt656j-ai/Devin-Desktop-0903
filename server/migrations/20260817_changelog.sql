-- The public changelog, editable from the console.
--
-- It started life as a TypeScript file in the website repo, which meant every entry cost a
-- rebuild and a deploy — fine for the developer who has the repo open, useless for writing
-- up a change the moment it ships. Moving it here is what makes "add" and "delete" real
-- operations rather than commits.
--
-- Entry text is authored prose, NOT release metadata. The distinction matters: an earlier
-- version of this page listed GitHub releases and printed the same auto-generated sentence
-- six times, because that is what release bodies contain when nobody writes them.
CREATE TABLE IF NOT EXISTS changelog_entries (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- The day the change shipped, as a date rather than a timestamp: a changelog is read
    -- by day, and storing an instant invites the timezone bug the website page already hit
    -- once (a UTC midnight rendering as the previous day west of Greenwich).
    entry_date DATE NOT NULL,

    -- Which part of the product. Free text so a new surface does not need a migration,
    -- bounded and trimmed by the handler.
    product    TEXT NOT NULL,
    title      TEXT NOT NULL,
    -- Optional version, when the change rides a release.
    version    TEXT NOT NULL DEFAULT '',

    -- [{ "kind": "added" | "fixed" | "changed", "text": "..." }, ...]
    -- JSONB rather than a child table: these are always read and written as a whole entry,
    -- never queried across, so a join would buy nothing and cost a second write path.
    changes    JSONB NOT NULL DEFAULT '[]'::jsonb,

    -- Unpublished entries are invisible to the public feed. Lets an entry be drafted in the
    -- console before the change is actually live, which is the one thing the file-based
    -- version could not do at all.
    published  BOOLEAN NOT NULL DEFAULT true,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The public feed reads published entries newest first, and nothing else.
CREATE INDEX IF NOT EXISTS idx_changelog_public
    ON changelog_entries (entry_date DESC, created_at DESC)
    WHERE published;
