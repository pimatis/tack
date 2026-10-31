-- backlog status, estimates/timers, parent-child hierarchy, task relations and
-- saved views. the two plain CREATE TABLEs come first on purpose: on a replay
-- (a downgraded binary drops the tracking row) the first statement fails with
-- "already exists", which the runner reads as "migration applied" and stops,
-- so the tasks rebuild below never runs twice
CREATE TABLE saved_views (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    filters TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE task_relations (
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    related_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    type TEXT NOT NULL CHECK (type IN ('blocks', 'relates', 'duplicates')),
    created_at TEXT NOT NULL,
    PRIMARY KEY (task_id, related_task_id, type)
);

-- teardown before rebuild: sqlite cannot alter a CHECK constraint, so tasks is
-- rebuilt below. the rebuild drops `tasks` for a window, and while it is gone
-- ALTER TABLE reparses dependent schema objects, so anything still pointing
-- at `tasks` or `task_search_content` in that window aborts the batch. every
-- fts trigger and the view come down first and are recreated at the end
DROP TRIGGER IF EXISTS tasks_fts_ai;
DROP TRIGGER IF EXISTS tasks_fts_au;
DROP TRIGGER IF EXISTS tasks_fts_ad;
DROP TRIGGER IF EXISTS subtasks_fts_ai;
DROP TRIGGER IF EXISTS subtasks_fts_au;
DROP TRIGGER IF EXISTS subtasks_fts_ad;
DROP TRIGGER IF EXISTS task_labels_fts_ai;
DROP TRIGGER IF EXISTS task_labels_fts_ad;
DROP TRIGGER IF EXISTS labels_fts_au;
DROP TRIGGER IF EXISTS labels_fts_ad;
DROP TRIGGER IF EXISTS projects_fts_au;
DROP TRIGGER IF EXISTS settings_fts_ai;
DROP TRIGGER IF EXISTS settings_fts_au;
DROP VIEW IF EXISTS task_search_content;

-- a stray tasks_new from an interrupted/manual attempt would trip the
-- runner's "already exists" tolerance and skip the whole rebuild
DROP TABLE IF EXISTS tasks_new;

CREATE TABLE tasks_new (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT,
    status TEXT NOT NULL DEFAULT 'todo'
        CHECK (status IN ('backlog', 'todo', 'in_progress', 'done', 'canceled')),
    priority INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 4),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    due_date TEXT,
    number INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    deleted_at TEXT,
    end_date TEXT,
    reminder_at TEXT,
    reminder_sent_at TEXT,
    recurrence_rule TEXT,
    recurrence_parent_id TEXT,
    parent_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    estimate_minutes INTEGER,
    tracked_seconds INTEGER NOT NULL DEFAULT 0,
    timer_started_at TEXT
);

INSERT INTO tasks_new (
    id, title, description, status, priority, created_at, updated_at,
    project_id, due_date, number, sort_order, pinned, deleted_at, end_date,
    reminder_at, reminder_sent_at, recurrence_rule, recurrence_parent_id
)
SELECT
    id, title, description, status, priority, created_at, updated_at,
    project_id, due_date, number, sort_order, pinned, deleted_at, end_date,
    reminder_at, reminder_sent_at, recurrence_rule, recurrence_parent_id
FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

CREATE INDEX IF NOT EXISTS idx_tasks_status_priority ON tasks (status, priority);
CREATE INDEX IF NOT EXISTS idx_tasks_updated_at ON tasks (updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks (project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_parent_id ON tasks (parent_id) WHERE parent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_reminder_at ON tasks (reminder_at) WHERE reminder_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_recurrence_parent ON tasks (recurrence_parent_id) WHERE recurrence_parent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_relations_task ON task_relations (task_id);
CREATE INDEX IF NOT EXISTS idx_relations_related ON task_relations (related_task_id);
CREATE INDEX IF NOT EXISTS idx_saved_views_sort ON saved_views (sort_order);

-- rebuild the search surface verbatim from migration 020 on the new table
CREATE VIEW task_search_content AS
WITH pad(n) AS (
    SELECT CAST(COALESCE((SELECT value FROM settings WHERE key = 'prefixPadding'), '0') AS INTEGER)
)
SELECT
    t.id AS task_id,
    t.title
    || ' ' || COALESCE(t.description, '')
    || ' ' || COALESCE(p.prefix || '-' || t.number, CAST(t.number AS TEXT))
    || ' ' || CAST(t.number AS TEXT)
    || ' ' || COALESCE(p.prefix || '-', '') || printf('%0*d', pad.n, t.number)
    || ' ' || COALESCE(p.name, '')
    || ' ' || COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), '')
    || ' ' || COALESCE((SELECT GROUP_CONCAT(l.name, ' ') FROM task_labels tl JOIN labels l ON l.id = tl.label_id WHERE tl.task_id = t.id), '')
    AS content_text
FROM tasks t
LEFT JOIN projects p ON p.id = t.project_id
CROSS JOIN pad;

CREATE TRIGGER tasks_fts_ai AFTER INSERT ON tasks BEGIN
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content WHERE task_id = NEW.id;
END;

CREATE TRIGGER tasks_fts_au AFTER UPDATE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content WHERE task_id = NEW.id;
END;

CREATE TRIGGER tasks_fts_ad AFTER DELETE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.id;
END;

CREATE TRIGGER subtasks_fts_ai AFTER INSERT ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = NEW.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content WHERE task_id = NEW.task_id;
END;

CREATE TRIGGER subtasks_fts_au AFTER UPDATE OF title ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = NEW.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content WHERE task_id = NEW.task_id;
END;

CREATE TRIGGER subtasks_fts_ad AFTER DELETE ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content WHERE task_id = OLD.task_id;
END;

CREATE TRIGGER task_labels_fts_ai AFTER INSERT ON task_labels BEGIN
    DELETE FROM tasks_fts WHERE task_id = NEW.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content WHERE task_id = NEW.task_id;
END;

CREATE TRIGGER task_labels_fts_ad AFTER DELETE ON task_labels BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content WHERE task_id = OLD.task_id;
END;

CREATE TRIGGER labels_fts_au AFTER UPDATE OF name ON labels BEGIN
    DELETE FROM tasks_fts WHERE task_id IN (SELECT task_id FROM task_labels WHERE label_id = NEW.id);
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content
    WHERE task_id IN (SELECT task_id FROM task_labels WHERE label_id = NEW.id);
END;

CREATE TRIGGER labels_fts_ad AFTER DELETE ON labels BEGIN
    DELETE FROM tasks_fts;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content;
END;

CREATE TRIGGER projects_fts_au AFTER UPDATE OF prefix, name ON projects BEGIN
    DELETE FROM tasks_fts WHERE task_id IN (SELECT id FROM tasks WHERE project_id = NEW.id);
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content
    WHERE task_id IN (SELECT id FROM tasks WHERE project_id = NEW.id);
END;

CREATE TRIGGER settings_fts_ai AFTER INSERT ON settings WHEN NEW.key = 'prefixPadding' BEGIN
    DELETE FROM tasks_fts;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content;
END;

CREATE TRIGGER settings_fts_au AFTER UPDATE OF value ON settings WHEN NEW.key = 'prefixPadding' BEGIN
    DELETE FROM tasks_fts;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content;
END;

-- the fts rows survived the rebuild (task ids are unchanged), but the copy is
-- cheap insurance in case a partial failure left it out of sync
DELETE FROM tasks_fts;
INSERT INTO tasks_fts (content_text, task_id)
SELECT content_text, task_id FROM task_search_content;
