CREATE TABLE tournament_teams (
  id CHAR(36) NOT NULL,
  tournament_id CHAR(36) NOT NULL,
  team_id CHAR(36) NOT NULL,
  group_id CHAR(36) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY tournament_teams_unique (tournament_id, team_id),
  KEY tournament_teams_group_idx (tournament_id, group_id),
  CONSTRAINT tournament_teams_tournament_id_fk FOREIGN KEY (tournament_id)
    REFERENCES tournaments (id) ON DELETE CASCADE,
  CONSTRAINT tournament_teams_team_id_fk FOREIGN KEY (team_id)
    REFERENCES teams (id) ON DELETE RESTRICT,
  CONSTRAINT tournament_teams_group_id_fk FOREIGN KEY (group_id)
    REFERENCES tournament_groups (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
