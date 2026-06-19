ALTER TABLE pools
  DROP COLUMN visibility,
  DROP COLUMN ranking_public,
  ADD COLUMN join_requires_allowlist BOOLEAN NOT NULL DEFAULT FALSE;
