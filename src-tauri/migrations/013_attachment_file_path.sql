-- migrate file_data TEXT to file_path TEXT (files now stored on disk)
ALTER TABLE task_attachments RENAME COLUMN file_data TO file_path;
