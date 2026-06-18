CREATE TABLE pools (
  id CHAR(36) NOT NULL,
  tournament_id CHAR(36) NOT NULL,
  owner_user_id CHAR(36) NOT NULL,
  name VARCHAR(140) NOT NULL,
  invite_code VARCHAR(20) NOT NULL,
  visibility ENUM('public','private') NOT NULL DEFAULT 'private',
  ranking_public BOOLEAN NOT NULL DEFAULT TRUE,
  prediction_lock_offset_minutes SMALLINT UNSIGNED NOT NULL DEFAULT 0,
  status ENUM('active','archived') NOT NULL DEFAULT 'active',
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY pools_invite_code_unique (invite_code),
  KEY pools_tournament_idx (tournament_id),
  KEY pools_owner_user_id_idx (owner_user_id),
  CONSTRAINT pools_tournament_id_fk FOREIGN KEY (tournament_id)
    REFERENCES tournaments (id) ON DELETE RESTRICT,
  CONSTRAINT pools_owner_user_id_fk FOREIGN KEY (owner_user_id)
    REFERENCES users (id) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
