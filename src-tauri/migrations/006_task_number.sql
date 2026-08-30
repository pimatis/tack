ALTER TABLE tasks ADD COLUMN number INTEGER NOT NULL DEFAULT 0;

UPDATE tasks SET number = (
    SELECT COUNT(*) FROM tasks t2
    WHERE t2.rowid <= tasks.rowid
    AND (
        (t2.project_id = tasks.project_id)
        OR (t2.project_id IS NULL AND tasks.project_id IS NULL)
    )
);

-- strip embedded project prefix from existing titles (e.g., "TSK - test" -> "test")
UPDATE tasks
SET title = substr(title, length((SELECT prefix FROM projects WHERE id = tasks.project_id)) + 4)
WHERE project_id IS NOT NULL
  AND title LIKE ((SELECT prefix FROM projects WHERE id = tasks.project_id) || ' - %');
