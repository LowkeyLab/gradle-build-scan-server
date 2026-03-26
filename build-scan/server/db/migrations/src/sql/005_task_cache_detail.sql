-- up_to_date_messages: JSON array of strings, NULL when absent (never empty array)
-- origin_build_invocation_id: build invocation that produced the cached result
ALTER TABLE tasks ADD COLUMN up_to_date_messages TEXT;
ALTER TABLE tasks ADD COLUMN origin_build_invocation_id TEXT;
