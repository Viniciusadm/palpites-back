ALTER TABLE users
  ADD COLUMN role ENUM('member','admin') NOT NULL DEFAULT 'member' AFTER display_name,
  ADD COLUMN avatar_file_id CHAR(36) NULL AFTER role,
  ADD INDEX users_role_idx (role);
