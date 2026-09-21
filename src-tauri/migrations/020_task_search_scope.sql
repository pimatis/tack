-- richer task search: index label names, project name/prefix and the issue
-- number alongside title, description and subtasks. a single view defines the
-- indexed text so the triggers below stay small; each trigger reindexes the
-- affected task whenever one of its inputs changes

DROP TRIGGER IF EXISTS tasks_fts_ai;
DROP TRIGGER IF EXISTS tasks_fts_au;
DROP TRIGGER IF EXISTS tasks_fts_ad;
DROP TRIGGER IF EXISTS subtasks_fts_ai;
DROP TRIGGER IF EXISTS subtasks_fts_au;
DROP TRIGGER IF EXISTS subtasks_fts_ad;

DROP VIEW IF EXISTS task_search_content;

-- the zero-padded form follows the app's prefixPadding setting, so a search
-- for the issue number matches exactly what the ui displays
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

-- label assignment changes the task's label text. deleting a label also
-- cascades through task_labels and fires here, but while the label row is still
-- visible, so labels_fts_ad rebuilds afterwards to drop the stale name
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

-- renaming a label or a project rewrites the text of every task using it
CREATE TRIGGER labels_fts_au AFTER UPDATE OF name ON labels BEGIN
    DELETE FROM tasks_fts WHERE task_id IN (SELECT task_id FROM task_labels WHERE label_id = NEW.id);
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT content_text, task_id FROM task_search_content
    WHERE task_id IN (SELECT task_id FROM task_labels WHERE label_id = NEW.id);
END;

-- a deleted label leaves no task_labels rows behind to target, and its
-- cascade fires while the label row is still visible, so rebuild wholesale
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

-- changing prefixPadding changes the displayed issue number for every task
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

-- rebuild the index with the new scope
DELETE FROM tasks_fts;
INSERT INTO tasks_fts (content_text, task_id)
SELECT content_text, task_id FROM task_search_content;
