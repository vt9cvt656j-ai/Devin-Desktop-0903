-- Optional admin-written blurb shown in the IDE model picker's hover info card
-- (alongside the auto-derived provider / model-id / input·output prices). Empty = the
-- card just shows the auto info.
ALTER TABLE models ADD COLUMN IF NOT EXISTS description TEXT NOT NULL DEFAULT '';
