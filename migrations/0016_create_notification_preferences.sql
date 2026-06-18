CREATE TABLE notification_preferences (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  pool_id CHAR(36) NULL,
  type ENUM('new_match','match_result','prediction_reminder','ranking_update','member_joined') NOT NULL,
  channel ENUM('in_app','email','push') NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY notification_preferences_unique (user_id, pool_id, type, channel),
  CONSTRAINT notification_preferences_user_id_fk FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT notification_preferences_pool_id_fk FOREIGN KEY (pool_id)
    REFERENCES pools (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
