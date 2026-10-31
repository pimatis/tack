-- recurring tasks. recurrence_rule is a compact schedule: daily,
-- weekly:<iso weekday 1-7> or monthly:<day 1-31>. completing a recurring task
-- spawns the next occurrence as a fresh todo, which points back to the task it
-- came from through recurrence_parent_id
--
-- these alters cannot be replayed, so they stay the last statements of the
-- file and the index lives in 023 where a replay is harmless
ALTER TABLE tasks ADD COLUMN recurrence_rule TEXT;
ALTER TABLE tasks ADD COLUMN recurrence_parent_id TEXT;
