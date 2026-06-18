CREATE TABLE tournament_stages (
  id CHAR(36) NOT NULL,
  tournament_id CHAR(36) NOT NULL,
  name VARCHAR(60) NOT NULL,
  kind ENUM('group','knockout') NOT NULL,
  ordering SMALLINT UNSIGNED NOT NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY tournament_stages_name_unique (tournament_id, name),
  KEY tournament_stages_ordering_idx (tournament_id, ordering),
  CONSTRAINT tournament_stages_tournament_id_fk FOREIGN KEY (tournament_id)
    REFERENCES tournaments (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
