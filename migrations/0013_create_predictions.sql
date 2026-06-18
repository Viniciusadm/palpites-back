CREATE TABLE predictions (
  id CHAR(36) NOT NULL,
  pool_member_id CHAR(36) NOT NULL,
  match_id CHAR(36) NOT NULL,
  home_score TINYINT UNSIGNED NOT NULL,
  away_score TINYINT UNSIGNED NOT NULL,
  points_awarded SMALLINT NULL,
  scored_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY predictions_member_match_unique (pool_member_id, match_id),
  KEY predictions_match_idx (match_id),
  KEY predictions_pool_member_idx (pool_member_id),
  CONSTRAINT predictions_pool_member_id_fk FOREIGN KEY (pool_member_id)
    REFERENCES pool_members (id) ON DELETE CASCADE,
  CONSTRAINT predictions_match_id_fk FOREIGN KEY (match_id)
    REFERENCES matches (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
