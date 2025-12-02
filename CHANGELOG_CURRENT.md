# Current Release Changelog

## Version: Security & Validation Enhancements

### Release Date
December 2024

### Summary
Comprehensive security and input validation improvements for RustGPT. This release focuses on protecting user accounts and preventing common attack vectors through password hashing and input validation.

## Major Features

### 🔐 Password Security (NEW)
- **BCrypt Integration**: All passwords now hashed with BCrypt (cost factor 12)
- **Password Change Endpoint**: Users can change passwords at `/settings/password`
- **Secure Password Verification**: Constant-time comparison prevents timing attacks
- **Database Schema**: Added fields for tracking password changes and account lockout

### ✅ Input Validation (NEW)
- **Authentication**: Email format, password length validation
- **Chat Operations**: Message length limits (max 10,000 chars)
- **Settings**: API key validation and minimum length requirements
- **Providers**: URL format validation, API key requirements
- **Agents**: Parameter bounds validation, name validation

### 🛡️ Security Improvements
- API key validation on update
- User authentication checks on sensitive operations
- Proper HTTP status codes for error cases
- Detailed error messages for debugging

## File Changes

### Modified Files
```
Cargo.toml                              - Added dependencies
src/utils/mod.rs                        - Password hashing utilities
src/router/app/auth.rs                  - Password hashing in signup/login
src/router/app/settings.rs              - Password change endpoint
src/router/app/chat.rs                  - Message validation
src/router/app/providers.rs             - Provider creation validation
src/router/app/agents.rs                - Agent creation validation
src/data/model.rs                       - DTO validation methods
src/router/app/mod.rs                   - New password change route
```

### New Files
```
db/migrations/20241203000001_add_password_security.sql  - Security fields
IMPROVEMENTS.md                         - Detailed improvements
VALIDATION_RULES.md                     - Validation reference
SECURITY_MIGRATION.md                   - Migration guide
CHANGELOG_CURRENT.md                    - This file
```

## Dependencies Added

- `bcrypt = "0.16"` - Password hashing library
- `thiserror = "1.0"` - Error handling framework

## Database Changes

### New Migration: `20241203000001_add_password_security.sql`
Added columns to `users` table:
- `password_version` (INTEGER) - For future hash algorithm changes
- `last_password_change` (DATETIME) - Security audit trail
- `failed_login_attempts` (INTEGER) - Account lockout tracking
- `locked_until` (DATETIME) - Account lockout expiration

Added indexes:
- `idx_users_email_password` - Faster email lookups

## API Changes

### New Endpoint
- `POST /settings/password` - Change user password
  - Requires: current_password, new_password, confirm_password
  - Returns: Redirect to /settings on success
  - Error codes: 400 (validation), 401 (unauthorized), 500 (database)

### Modified Endpoints
- `POST /login` - Now requires valid credentials against hashed passwords
- `POST /signup` - Passwords now automatically hashed
- `POST /settings` - API key validation added

## Breaking Changes

⚠️ **Important**: Existing plain-text passwords need migration.

Two options:
1. **Force Password Reset** (Recommended): All users reset passwords
2. **Manual Migration**: Run provided Python script to hash existing passwords

See `SECURITY_MIGRATION.md` for detailed instructions.

## Validation Rules Summary

### Authentication
- Email: Non-empty, must contain '@'
- Password: Min 8 chars for signup, non-empty for login
- Password confirmation: Must match during signup and password change

### Chat
- Message: Non-empty after trim, max 10,000 characters
- Model: Must be non-empty and selected

### Settings
- API Key: Non-empty, minimum 20 characters

### Providers
- Name: 1-100 characters
- Base URL: Must start with http:// or https://
- API Key: Minimum 10 characters

### Agents
- Name: 1-100 characters
- Provider ID: Must be > 0
- Numeric ranges:
  - temperature: 0.0 - 2.0
  - top_p: 0.0 - 1.0
  - max_tokens: 1 - 100,000
  - max_context: 1 - 1,000,000

## Performance Impact

- **Password operations**: ~100ms per hash/verify (intentional for security)
- **Validation**: Negligible (microseconds)
- **Database queries**: Slightly faster due to new indexes

## Migration Guide

### For Fresh Installations
1. Run: `just init`
2. Database automatically created with new schema
3. All functionality available immediately

### For Existing Installations
1. Run: `just db-migrate`
2. Choose migration strategy (see `SECURITY_MIGRATION.md`)
3. Test signup and login functionality
4. Notify users if password reset is required

## Testing Recommendations

### Manual Testing
- [ ] Signup with valid credentials
- [ ] Signup with invalid email format
- [ ] Signup with short password
- [ ] Login with correct credentials
- [ ] Login with wrong password
- [ ] Change password with current password verification
- [ ] Create chat with message
- [ ] Create chat with 10001+ character message
- [ ] Create provider with valid URL
- [ ] Create provider with invalid URL
- [ ] Create agent with valid parameters
- [ ] Create agent with out-of-range parameters

### Automated Testing
```bash
# Run tests (once setup is complete)
cargo test
```

## Known Limitations

- Password reset functionality (email-based) not yet implemented
- Two-factor authentication not yet implemented
- Account lockout mechanism prepared but not active
- API key encryption at rest not yet implemented
- CSRF token support not yet implemented

## Documentation

New documentation files:
- `IMPROVEMENTS.md` - Detailed technical improvements
- `VALIDATION_RULES.md` - Complete validation reference
- `SECURITY_MIGRATION.md` - Database migration guide

## Future Enhancements

### Planned for Next Release
- [ ] Email-based password reset
- [ ] Two-factor authentication (2FA)
- [ ] Account lockout after N failed attempts
- [ ] Email verification for new signups
- [ ] API key encryption at rest
- [ ] Audit logging for security events

### Long-term Roadmap
- [ ] OAuth2/OIDC support
- [ ] Role-based access control (RBAC)
- [ ] Single sign-on (SSO)
- [ ] API rate limiting
- [ ] Session management dashboard
- [ ] Security policy enforcement

## Upgrade Instructions

### Step 1: Backup Database
```bash
cp db/db.db db/db.db.backup
```

### Step 2: Update Code
```bash
git pull
```

### Step 3: Run Migration
```bash
just db-migrate
```

### Step 4: Migrate Passwords
See `SECURITY_MIGRATION.md` for detailed instructions.

### Step 5: Restart Application
```bash
just dev
```

### Step 6: Verify
- Test user signup
- Test user login
- Test password change
- Check application logs for errors

## Credits

- Axum web framework for secure routing
- BCrypt for industry-standard password hashing
- SQLite for reliable data persistence

## Support

For questions or issues:
1. Check the relevant documentation file
2. Review error messages in logs
3. Test with fresh database: `just db-reset`
4. Contact support with detailed error messages

## License

Same as RustGPT main project

---

**Status**: Ready for testing and deployment
**Breaking Changes**: Yes (password migration required)
**Database Migration**: Required
**Backwards Compatibility**: Partial (see migration guide)
