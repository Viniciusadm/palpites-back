ALTER TABLE users
  ADD COLUMN sync_predictions_across_pools BOOLEAN NOT NULL DEFAULT TRUE;
