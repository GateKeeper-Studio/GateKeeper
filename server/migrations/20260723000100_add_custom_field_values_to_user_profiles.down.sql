-- Add down migration script here
ALTER TABLE user_profiles DROP COLUMN IF EXISTS custom_field_values;
