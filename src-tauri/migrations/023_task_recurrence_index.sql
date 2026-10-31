-- keeps the spawn lookup off a full table scan: a series has one occurrence per
-- due date, and completing a task probes exactly that pair
CREATE INDEX IF NOT EXISTS idx_tasks_recurrence_parent
    ON tasks (recurrence_parent_id) WHERE recurrence_parent_id IS NOT NULL;
