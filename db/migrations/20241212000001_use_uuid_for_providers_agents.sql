-- Add UUID columns to providers and agents tables
-- This migration transitions from integer IDs to UUIDs while maintaining backward compatibility

-- Add UUID columns to providers table
ALTER TABLE providers ADD COLUMN uuid TEXT;
ALTER TABLE providers ADD COLUMN is_legacy_id BOOLEAN DEFAULT TRUE;

-- Add UUID columns to agents table
ALTER TABLE agents ADD COLUMN uuid TEXT;
ALTER TABLE agents ADD COLUMN is_legacy_id BOOLEAN DEFAULT TRUE;

-- Create new UUID-based foreign key columns
ALTER TABLE provider_models ADD COLUMN provider_uuid TEXT;
ALTER TABLE agents ADD COLUMN user_uuid TEXT;
ALTER TABLE agents ADD COLUMN provider_uuid TEXT;

-- Create indexes for UUID columns
CREATE INDEX idx_providers_uuid ON providers(uuid) WHERE uuid IS NOT NULL;
CREATE INDEX idx_agents_uuid ON agents(uuid) WHERE uuid IS NOT NULL;
CREATE INDEX idx_provider_models_provider_uuid ON provider_models(provider_uuid) WHERE provider_uuid IS NOT NULL;

-- Note: Unique constraints for UUIDs will be handled at application level
-- SQLite doesn't support partial unique constraints with WHERE clauses

-- Add comment about migration purpose
-- This migration allows gradual transition from integer IDs to UUIDs
-- Legacy records will have is_legacy_id = TRUE and no UUID initially
-- New records should have is_legacy_id = FALSE and a generated UUID