-- Add migration script here
ALTER TABLE chores ADD COLUMN on_cadence INTEGER NOT NULL DEFAULT 0;
