CREATE TABLE pool_email_allowlist (
  id CHAR(36) NOT NULL,
  pool_id CHAR(36) NOT NULL,
  email VARCHAR(255) NOT NULL,
  added_by_user_id CHAR(36) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY pool_email_allowlist_unique (pool_id, email),
  KEY pool_email_allowlist_pool_idx (pool_id),
  CONSTRAINT pool_email_allowlist_pool_id_fk FOREIGN KEY (pool_id)
    REFERENCES pools (id) ON DELETE CASCADE,
  CONSTRAINT pool_email_allowlist_user_id_fk FOREIGN KEY (added_by_user_id)
    REFERENCES users (id) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
