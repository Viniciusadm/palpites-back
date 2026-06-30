ALTER TABLE matches
  ADD COLUMN can_go_to_penalties BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN penalties_winner ENUM('home','away') NULL;
