CREATE TABLE notifications (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  pool_id CHAR(36) NULL,
  type ENUM('new_match','match_result','prediction_reminder','ranking_update','member_joined','invite') NOT NULL,
  title VARCHAR(160) NOT NULL,
  body VARCHAR(500) NOT NULL,
  related_match_id CHAR(36) NULL,
  read_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY notifications_user_read_idx (user_id, read_at),
  KEY notifications_user_created_idx (user_id, created_at),
  CONSTRAINT notifications_user_id_fk FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT notifications_pool_id_fk FOREIGN KEY (pool_id)
    REFERENCES pools (id) ON DELETE CASCADE,
  CONSTRAINT notifications_related_match_id_fk FOREIGN KEY (related_match_id)
    REFERENCES matches (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
