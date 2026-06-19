CREATE TABLE admin_audit_logs (
  id CHAR(36) NOT NULL,
  user_id CHAR(36) NULL,
  method VARCHAR(10) NOT NULL,
  path VARCHAR(512) NOT NULL,
  query_string VARCHAR(1024) NULL,
  request_body JSON NULL,
  status_code SMALLINT UNSIGNED NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY admin_audit_logs_user_id_idx (user_id),
  KEY admin_audit_logs_created_at_idx (created_at),
  CONSTRAINT admin_audit_logs_user_id_fk FOREIGN KEY (user_id)
    REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
