-- Per-user Skills library (reusable AI prompts the user creates in the IDE).
-- One row per user; the whole list is stored as a JSON array and replaced on save.
CREATE TABLE IF NOT EXISTS user_skills (
    user_id    uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    skills     jsonb NOT NULL DEFAULT '[]'::jsonb,
    updated_at timestamptz NOT NULL DEFAULT now()
);
