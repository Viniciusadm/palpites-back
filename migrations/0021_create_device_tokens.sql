CREATE TABLE device_tokens (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  token VARCHAR(512) NOT NULL,
  platform VARCHAR(32) NOT NULL DEFAULT 'web',
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  last_seen_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY device_tokens_token_unique (token),
  KEY device_tokens_user_idx (user_id),
  CONSTRAINT device_tokens_user_id_fk FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
