ALTER TABLE pool_scoring_rules
  MODIFY rule_key ENUM('exact_score','correct_outcome','correct_goal_difference','penalties_winner') NOT NULL;

ALTER TABLE pool_standings
  ADD COLUMN penalties_count INT NOT NULL DEFAULT 0;

-- Backfill the new scoring rule (default 5 points) for every existing pool so the
-- penalties feature is active everywhere, not just on pools created from now on.
INSERT INTO pool_scoring_rules (id, pool_id, rule_key, points)
SELECT UUID(), id, 'penalties_winner', 5
FROM pools;
