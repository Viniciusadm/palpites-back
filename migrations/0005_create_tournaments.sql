CREATE TABLE tournaments (
  id CHAR(36) NOT NULL,
  name VARCHAR(140) NOT NULL,
  slug VARCHAR(160) NOT NULL,
  season_year SMALLINT UNSIGNED NULL,
  starts_on DATE NULL,
  ends_on DATE NULL,
  status ENUM('draft','active','finished','archived') NOT NULL DEFAULT 'draft',
  created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  UNIQUE KEY tournaments_slug_unique (slug),
  KEY tournaments_status_idx (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
