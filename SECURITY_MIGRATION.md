# Security Migration Guide

This guide helps you migrate an existing RustGPT database to use the new password security features.

## Overview

The latest update introduces:
1. **BCrypt password hashing** (replacing plain-text storage)
2. **Input validation** on all forms and API endpoints
3. **Password security fields** in the database

## Database Migration

### Automatic Migration
The database migration is automatically applied when you run:
```bash
just db-migrate
```

Or manually:
```bash
sqlx migrate run
```

### What the Migration Does
- Adds `password_version` column (integer, default 1)
- Adds `last_password_change` column (datetime)
- Adds `failed_login_attempts` column (integer, default 0)
- Adds `locked_until` column (datetime, nullable)
- Creates index on `email` column for faster lookups

### Breaking Change ⚠️

**Important**: Existing passwords are stored as **plain text**. This is a security issue.

After the migration, you have two options:

## Option A: Force Password Reset (Recommended)

This is the safest approach:

1. Run the migration:
   ```bash
   just db-migrate
   ```

2. Notify all users to reset their passwords

3. Optional: Create a redirect for unauthenticated users to reset password

**Advantages**:
- All passwords are new hashes
- Users verify they control their email
- Clean security state

**Disadvantages**:
- Users must reset passwords
- Some users might get locked out

## Option B: Manual Migration Script

If you want to preserve existing passwords:

```python
#!/usr/bin/env python3
import sqlite3
import bcrypt

# Connect to database
conn = sqlite3.connect('db/db.db')
cursor = conn.cursor()

# Get all users with plain-text passwords
cursor.execute('SELECT id, password FROM users')
users = cursor.fetchall()

for user_id, plain_password in users:
    # Hash the password
    hashed = bcrypt.hashpw(plain_password.encode(), bcrypt.gensalt(rounds=12))
    
    # Update in database
    cursor.execute(
        'UPDATE users SET password = ? WHERE id = ?',
        (hashed.decode(), user_id)
    )

conn.commit()
conn.close()
print(f"Migrated {len(users)} passwords to BCrypt hashing")
```

### Using the Migration Script

1. Save the script as `migrate_passwords.py`
2. Run it:
   ```bash
   python3 migrate_passwords.py
   ```
3. Verify no errors occurred
4. Restart the application

**Advantages**:
- No user disruption
- Existing credentials continue to work
- Seamless migration

**Disadvantages**:
- Requires manual script execution
- Old passwords were stored in plain text

## Updating Application Code

The application automatically uses password hashing for:
- All new signups
- All password changes
- All login attempts

**No code changes required** - the application handles hashing transparently.

## Updating Frontend/Documentation

### User Communication

If you're using **Option A** (force reset):
```
Subject: Important Security Update - Please Reset Your Password

We've implemented new security features for RustGPT. For your safety,
please reset your password at: https://yoursite.com/settings/password

This ensures your account uses the latest encryption standards.
```

If you're using **Option B** (seamless migration):
```
Subject: RustGPT Security Improvements

We've updated our security measures. Your account will automatically
use the new encryption on your next login or password change.
```

## Verification Steps

### 1. Test Login with Old Credentials
After migration, verify that:
```bash
# Login should work if using Option B
# Or fail with message to reset password if using Option A
```

### 2. Test Signup with New Credentials
```bash
# Create new test account
# Verify password is hashed (not plain text in database)
```

### 3. Test Password Change
```bash
# Use /settings/password endpoint
# Verify new password is hashed
```

### 4. Database Verification

Check that passwords are now hashed:
```sql
-- These should show hashed values starting with $2a$, $2b$, or $2x$
SELECT email, SUBSTR(password, 1, 20) as password_hash FROM users LIMIT 5;
```

Hashed password example:
```
$2b$12$8xF0oP3rEk8xK5vF9pL2uOqQcM3nJ9xK5vF9pL2uOqQcM3nJ9xK5v
```

## Performance Considerations

### BCrypt Hashing Cost

With `cost = 12` (default):
- **Signup**: ~100ms per password hash
- **Login**: ~100ms per password verification
- **Password Change**: ~100ms per hash

This is **intentional** - the delay makes brute-force attacks impractical.

### Optimization Options

If performance is critical, you can adjust the cost (in `src/utils/mod.rs`):

```rust
// Lower cost = faster, but less secure
bcrypt::hash(password, 10)  // ~40ms per operation
bcrypt::hash(password, 12)  // ~100ms per operation (default)
bcrypt::hash(password, 14)  // ~300ms per operation
```

**Recommendation**: Keep default (12) for balance of security and performance.

## Rollback Plan

If something goes wrong:

### 1. Rollback Database
```bash
# Reset database to previous state
just db-reset
```

### 2. Checkout Previous Code
```bash
git checkout HEAD~1
```

### 3. Investigate Issues
Check logs for errors:
```bash
# View application logs
tail -f logs/app.log

# View error details
grep -i error logs/app.log
```

## Testing Checklist

- [ ] Database migration runs without errors
- [ ] Existing users can still login (with appropriate migration)
- [ ] New signups create hashed passwords
- [ ] Password changes work correctly
- [ ] Failed logins show appropriate error
- [ ] Settings page loads correctly
- [ ] No SQL errors in logs
- [ ] Performance is acceptable

## Support

If you encounter issues:

1. Check the application logs
2. Verify database migration ran successfully:
   ```sql
   PRAGMA table_info(users);  -- Should show new columns
   ```
3. Test with a fresh database:
   ```bash
   just db-reset
   ```
4. Contact support with error messages

## Additional Resources

- [BCrypt Documentation](https://en.wikipedia.org/wiki/Bcrypt)
- [OWASP Password Storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [RustGPT Documentation](./README.md)
- [Validation Rules](./VALIDATION_RULES.md)
- [Improvements Summary](./IMPROVEMENTS.md)
