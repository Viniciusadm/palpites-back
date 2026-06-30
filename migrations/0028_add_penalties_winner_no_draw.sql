-- New scoring category: correctly calling the penalty-shootout winner WITHOUT
-- having predicted the draw (predicted a decisive result). Separate from the
-- existing `penalties_winner` (which requires predicting the draw).
ALTER TABLE pool_scoring_rules
  MODIFY rule_key ENUM(
    'exact_score',
    'correct_outcome',
    'correct_goal_difference',
    'penalties_winner',
    'penalties_winner_no_draw'
  ) NOT NULL;

ALTER TABLE pool_standings
  ADD COLUMN penalties_no_draw_count INT NOT NULL DEFAULT 0;

-- Backfill the new rule (default 2 points) for every existing pool so the new
-- category is active everywhere, not just on pools created from now on.
INSERT INTO pool_scoring_rules (id, pool_id, rule_key, points)
SELECT UUID(), id, 'penalties_winner_no_draw', 2
FROM pools;

-- Idempotency marker for one-off data backfills run by the application at
-- startup (see rescore step). The row is claimed once and never re-run.
CREATE TABLE IF NOT EXISTS applied_backfills (
  name VARCHAR(190) NOT NULL,
  applied_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
