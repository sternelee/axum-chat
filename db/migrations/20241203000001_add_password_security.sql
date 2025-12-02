-- Add password security improvements
-- Note: This migration prepares the schema for password hashing
-- Existing plain-text passwords should be migrated separately

-- Add a new column for tracking password version (for future hash algorithm changes)
ALTER TABLE users ADD COLUMN password_version INTEGER DEFAULT 1;

-- Add last password change timestamp for security auditing
ALTER TABLE users ADD COLUMN last_password_change DATETIME DEFAULT CURRENT_TIMESTAMP;

-- Add account lock mechanism for failed login attempts
ALTER TABLE users ADD COLUMN failed_login_attempts INTEGER DEFAULT 0;
ALTER TABLE users ADD COLUMN locked_until DATETIME;

-- Create an index for faster password lookups
CREATE INDEX idx_users_email_password ON users(email);
