-- per-task reminders. reminder_at is a utc iso timestamp (the moment to
-- notify); reminder_sent_at records that the notification already fired so a
-- reminder triggers exactly once (NULL until then, reset when reminder_at
-- changes)
ALTER TABLE tasks ADD COLUMN reminder_at TEXT;
ALTER TABLE tasks ADD COLUMN reminder_sent_at TEXT;

CREATE INDEX IF NOT EXISTS idx_tasks_reminder_at
    ON tasks (reminder_at) WHERE reminder_at IS NOT NULL;
