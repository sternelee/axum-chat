-- Populate UUID values for existing providers and agents
-- This migration generates UUIDs for all records that don't have them yet

-- Update existing providers with UUIDs and set legacy flag
UPDATE providers
SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-' || '4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab',1 + (abs(random()) % 4), 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
    is_legacy_id = TRUE
WHERE uuid IS NULL;

-- Update existing agents with UUIDs and set legacy flag
UPDATE agents
SET uuid = lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-' || '4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab',1 + (abs(random()) % 4), 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
    is_legacy_id = TRUE
WHERE uuid IS NULL;