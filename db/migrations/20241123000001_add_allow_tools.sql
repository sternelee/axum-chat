-- Add allow_tools field to agents table
ALTER TABLE agents ADD COLUMN allow_tools TEXT DEFAULT '[]'; -- JSON array of tool IDs that are auto-approved