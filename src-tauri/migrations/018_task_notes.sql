-- explicit links from a task to a note, added from the task panel; the
-- reverse direction (note -> task) is derived from note content and lives in
-- the note_links index instead
CREATE TABLE IF NOT EXISTS task_notes (
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    note_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (task_id, note_path)
);

CREATE INDEX IF NOT EXISTS idx_task_notes_task_id ON task_notes (task_id);
