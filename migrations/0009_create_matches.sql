CREATE TABLE matches (
  id CHAR(36) NOT NULL,
  tournament_id CHAR(36) NOT NULL,
  stage_id CHAR(36) NOT NULL,
  home_team_id CHAR(36) NULL,
  away_team_id CHAR(36) NULL,
  kickoff_at DATETIME(6) NOT NULL,
  status ENUM('scheduled','live','finished','canceled') NOT NULL DEFAULT 'scheduled',
  home_score TINYINT UNSIGNED NULL,
  away_score TINYINT UNSIGNED NULL,
  finished_at DATETIME(6) NULL,
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  KEY matches_tournament_kickoff_idx (tournament_id, kickoff_at),
  KEY matches_status_idx (status),
  KEY matches_stage_idx (stage_id),
  KEY matches_home_team_idx (home_team_id),
  KEY matches_away_team_idx (away_team_id),
  CONSTRAINT matches_tournament_id_fk FOREIGN KEY (tournament_id)
    REFERENCES tournaments (id) ON DELETE CASCADE,
  CONSTRAINT matches_stage_id_fk FOREIGN KEY (stage_id)
    REFERENCES tournament_stages (id) ON DELETE RESTRICT,
  CONSTRAINT matches_home_team_id_fk FOREIGN KEY (home_team_id)
    REFERENCES teams (id) ON DELETE RESTRICT,
  CONSTRAINT matches_away_team_id_fk FOREIGN KEY (away_team_id)
    REFERENCES teams (id) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
