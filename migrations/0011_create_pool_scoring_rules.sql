CREATE TABLE pool_scoring_rules (
  id CHAR(36) NOT NULL,
  pool_id CHAR(36) NOT NULL,
  rule_key ENUM('exact_score','correct_outcome','correct_goal_difference') NOT NULL,
  points SMALLINT NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY pool_scoring_rules_unique (pool_id, rule_key),
  CONSTRAINT pool_scoring_rules_pool_id_fk FOREIGN KEY (pool_id)
    REFERENCES pools (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
