CREATE TABLE IF NOT EXISTS chores
(
  id                       INTEGER PRIMARY KEY NOT NULL,
  display_name             TEXT    UNIQUE NOT NULL,
  frequency_hours          INTEGER,
  last_completed_at        INTEGER
);
