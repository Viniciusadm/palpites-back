CREATE TABLE pool_standings (
  id CHAR(36) NOT NULL,
  pool_id CHAR(36) NOT NULL,
  pool_member_id CHAR(36) NOT NULL,
  total_points INT NOT NULL DEFAULT 0,
  exact_count INT NOT NULL DEFAULT 0,
  outcome_count INT NOT NULL DEFAULT 0,
  hits_count INT NOT NULL DEFAULT 0,
  position INT NOT NULL DEFAULT 0,
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY pool_standings_pool_member_unique (pool_id, pool_member_id),
  KEY pool_standings_pool_points_idx (pool_id, total_points),
  KEY pool_standings_pool_position_idx (pool_id, position),
  CONSTRAINT pool_standings_pool_id_fk FOREIGN KEY (pool_id)
    REFERENCES pools (id) ON DELETE CASCADE,
  CONSTRAINT pool_standings_pool_member_id_fk FOREIGN KEY (pool_member_id)
    REFERENCES pool_members (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
