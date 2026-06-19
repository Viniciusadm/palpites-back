CREATE TABLE notification_outbox (
  id CHAR(36) NOT NULL,
  channel ENUM('push') NOT NULL,
  user_id CHAR(36) NOT NULL,
  title VARCHAR(160) NOT NULL,
  body VARCHAR(500) NOT NULL,
  related_match_id CHAR(36) NULL,
  pool_id CHAR(36) NULL,
  status ENUM('pending','delivered','failed') NOT NULL DEFAULT 'pending',
  attempts INT NOT NULL DEFAULT 0,
  last_error VARCHAR(500) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  delivered_at DATETIME(6) NULL,
  PRIMARY KEY (id),
  KEY notification_outbox_status_idx (status, created_at),
  CONSTRAINT notification_outbox_user_id_fk FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT notification_outbox_related_match_id_fk FOREIGN KEY (related_match_id)
    REFERENCES matches (id) ON DELETE SET NULL,
  CONSTRAINT notification_outbox_pool_id_fk FOREIGN KEY (pool_id)
    REFERENCES pools (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
