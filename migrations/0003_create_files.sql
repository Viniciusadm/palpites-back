CREATE TABLE files (
  id CHAR(36) NOT NULL,
  owner_user_id CHAR(36) NULL,
  bucket VARCHAR(100) NOT NULL,
  object_key VARCHAR(512) NOT NULL,
  content_type VARCHAR(120) NOT NULL,
  byte_size BIGINT UNSIGNED NOT NULL,
  original_name VARCHAR(255) NOT NULL,
  checksum_sha256 CHAR(64) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY files_bucket_object_key_unique (bucket, object_key),
  INDEX files_owner_user_id_idx (owner_user_id),
  CONSTRAINT files_owner_user_id_fk FOREIGN KEY (owner_user_id)
    REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE users
  ADD CONSTRAINT users_avatar_file_id_fk FOREIGN KEY (avatar_file_id)
    REFERENCES files (id) ON DELETE SET NULL;
