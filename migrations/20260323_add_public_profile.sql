-- AFD-132: Add public profile fields to users table
ALTER TABLE users
    ADD COLUMN username VARCHAR(30) UNIQUE NULL DEFAULT NULL AFTER display_name,
    ADD COLUMN profile_public BOOLEAN NOT NULL DEFAULT TRUE AFTER phone_number,
    ADD COLUMN privacy_settings JSON NULL DEFAULT NULL AFTER profile_public;

CREATE INDEX idx_users_username ON users (username);
