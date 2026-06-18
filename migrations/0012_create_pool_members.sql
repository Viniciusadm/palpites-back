CREATE TABLE pool_members (
  id CHAR(36) NOT NULL,
  pool_id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  role ENUM('owner','admin','member') NOT NULL DEFAULT 'member',
  status ENUM('active','inactive') NOT NULL DEFAULT 'active',
  joined_at DATETIME(6) NOT NULL,
  left_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY pool_members_unique (pool_id, user_id),
  KEY pool_members_pool_status_idx (pool_id, status),
  KEY pool_members_user_idx (user_id),
  CONSTRAINT pool_members_pool_id_fk FOREIGN KEY (pool_id)
    REFERENCES pools (id) ON DELETE CASCADE,
  CONSTRAINT pool_members_user_id_fk FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
