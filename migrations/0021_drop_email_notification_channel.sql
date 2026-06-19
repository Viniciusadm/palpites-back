DELETE FROM notification_preferences WHERE channel = 'email';
ALTER TABLE notification_preferences MODIFY COLUMN channel ENUM('in_app','push') NOT NULL;
