-- Compatibility epoch: retained research drafts require a source-checking reader.
-- No table or record bytes change. The migration owner atomically appends this
-- exact statement digest to schema_history and advances user_version to 20.
SELECT 1;
