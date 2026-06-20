ALTER TABLE notification_outbox
  ADD COLUMN type ENUM('new_match','match_result','prediction_reminder','ranking_update','member_joined','invite') NULL
    AFTER channel,
  ADD KEY notification_outbox_type_created_idx (type, created_at);
