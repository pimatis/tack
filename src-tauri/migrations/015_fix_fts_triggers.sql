-- repair malformed fts triggers from migration 014
-- the insert statements declared two columns but selected only one value,
-- which made every insert/update on tasks fail with "1 values for 2 columns"
DROP TRIGGER IF EXISTS tasks_fts_ai;
DROP TRIGGER IF EXISTS tasks_fts_au;
DROP TRIGGER IF EXISTS tasks_fts_ad;
DROP TRIGGER IF EXISTS subtasks_fts_ai;
DROP TRIGGER IF EXISTS subtasks_fts_au;
DROP TRIGGER IF EXISTS subtasks_fts_ad;

CREATE TRIGGER tasks_fts_ai AFTER INSERT ON tasks BEGIN
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), ''), t.id
    FROM tasks t WHERE t.id = NEW.id;
END;

CREATE TRIGGER tasks_fts_au AFTER UPDATE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), ''), t.id
    FROM tasks t WHERE t.id = NEW.id;
END;

CREATE TRIGGER tasks_fts_ad AFTER DELETE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.id;
END;

CREATE TRIGGER subtasks_fts_ai AFTER INSERT ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = NEW.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), ''), t.id
    FROM tasks t WHERE t.id = NEW.task_id;
END;

CREATE TRIGGER subtasks_fts_au AFTER UPDATE OF title ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = NEW.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), ''), t.id
    FROM tasks t WHERE t.id = NEW.task_id;
END;

CREATE TRIGGER subtasks_fts_ad AFTER DELETE ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), ''), t.id
    FROM tasks t WHERE t.id = OLD.task_id;
END;

-- rebuild fts content so tasks written while the triggers were broken are searchable
DELETE FROM tasks_fts;

INSERT INTO tasks_fts (content_text, task_id)
SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
       COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), ''), t.id
FROM tasks t;
