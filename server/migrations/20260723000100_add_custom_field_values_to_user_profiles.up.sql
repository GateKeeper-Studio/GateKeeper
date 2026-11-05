-- Add up migration script here
ALTER TABLE user_profiles ADD COLUMN custom_field_values JSONB NULL;
