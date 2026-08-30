-- full text search over task title, description and subtask titles
-- one fts row per task; content_text = title + description + subtask titles
CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts USING fts5(content_text, task_id UNINDEXED);

CREATE TRIGGER IF NOT EXISTS tasks_fts_ai AFTER INSERT ON tasks BEGIN
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), '')
    FROM tasks t WHERE t.id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS tasks_fts_au AFTER UPDATE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), '')
    FROM tasks t WHERE t.id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS tasks_fts_ad AFTER DELETE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS subtasks_fts_ai AFTER INSERT ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = NEW.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), '')
    FROM tasks t WHERE t.id = NEW.task_id;
END;

CREATE TRIGGER IF NOT EXISTS subtasks_fts_au AFTER UPDATE OF title ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = NEW.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), '')
    FROM tasks t WHERE t.id = NEW.task_id;
END;

CREATE TRIGGER IF NOT EXISTS subtasks_fts_ad AFTER DELETE ON subtasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = OLD.task_id;
    INSERT INTO tasks_fts (content_text, task_id)
    SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
           COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), '')
    FROM tasks t WHERE t.id = OLD.task_id;
END;

-- backfill existing tasks
INSERT INTO tasks_fts (content_text, task_id)
SELECT t.title || ' ' || COALESCE(t.description, '') || ' ' ||
       COALESCE((SELECT GROUP_CONCAT(s.title, ' ') FROM subtasks s WHERE s.task_id = t.id), ''), t.id
FROM tasks t;
