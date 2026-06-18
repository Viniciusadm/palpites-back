CREATE TABLE tournament_groups (
  id CHAR(36) NOT NULL,
  tournament_id CHAR(36) NOT NULL,
  label VARCHAR(8) NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY tournament_groups_label_unique (tournament_id, label),
  CONSTRAINT tournament_groups_tournament_id_fk FOREIGN KEY (tournament_id)
    REFERENCES tournaments (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
