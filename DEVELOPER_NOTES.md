# Developer Notes for RustGPT Security Updates

## Quick Reference

### What Changed?
This release adds password hashing and comprehensive input validation to RustGPT.

### Key Files Modified
- `src/utils/mod.rs` - Password utilities
- `src/router/app/auth.rs` - Auth handlers with password hashing
- `src/router/app/settings.rs` - New password change endpoint
- Other handlers - Input validation added

### Key Files Added
- `db/migrations/20241203000001_add_password_security.sql` - Database schema
- Documentation files

## Development Workflow

### Setup for Development
```bash
# Install dependencies and create database
just init

# Start development server
just dev

# Or run components separately
just dev-server  # Rust compilation with watch
just dev-tailwind  # Tailwind CSS watch
```

### Testing Changes
```bash
# Create a test account
# Navigate to http://localhost:3000/signup
# Email: test@example.com
# Password: TestPassword123

# Test login with credentials
# Verify password is hashed in database:
sqlite3 db/db.db "SELECT email, SUBSTR(password,1,20) FROM users;"
# Should see: test@example.com|$2b$12$...
```

### Database Operations
```bash
# View current schema
sqlite3 db/db.db ".schema users"

# Check new columns were added
sqlite3 db/db.db "PRAGMA table_info(users);"

# Reset database to clean state
just db-reset
```

## Code Style Guidelines

### Password Functions
```rust
use crate::utils::{hash_password, verify_password};

// Always use hash_password for new passwords
let hashed = hash_password(&password)?;

// Always use verify_password for validation
let is_valid = verify_password(&input, &stored_hash)?;
```

### Validation Methods
```rust
// DTOs should implement validate()
impl MyRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.field.is_empty() {
            return Err("Field cannot be empty".to_string());
        }
        Ok(())
    }
}

// Use in handlers
pub async fn handler(Json(req): Json<MyRequest>) -> Result<...> {
    req.validate().map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    // Continue with handler logic
}
```

### Error Handling
```rust
// Use proper error types
#[derive(Debug)]
pub enum MyError {
    ValidationFailed(String),
    Unauthorized,
    DatabaseError,
}

// Implement IntoResponse for custom errors
impl IntoResponse for MyError {
    fn into_response(self) -> Response {
        match self {
            MyError::ValidationFailed(msg) => {
                (StatusCode::BAD_REQUEST, Json(msg)).into_response()
            }
            // ... other cases
        }
    }
}
```

## Common Development Tasks

### Adding Password-Protected Endpoint
```rust
// 1. Add to route with auth middleware
.layer(axum::middleware::from_fn(auth))

// 2. Use Extension to get user
pub async fn my_handler(
    Extension(current_user): Extension<Option<User>>,
) -> Result<...> {
    let user = current_user.ok_or(MyError::Unauthorized)?;
    // Use user.id, user.email, etc.
}
```

### Adding Input Validation to New Form
```rust
// 1. Define struct
#[derive(Deserialize)]
pub struct MyForm {
    email: String,
    message: String,
}

// 2. Add validate method
impl MyForm {
    pub fn validate(&self) -> Result<(), String> {
        if self.email.is_empty() {
            return Err("Email required".to_string());
        }
        if self.message.len() > 1000 {
            return Err("Message too long".to_string());
        }
        Ok(())
    }
}

// 3. Call in handler
pub async fn handler(Form(form): Form<MyForm>) -> Result<...> {
    form.validate().map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    // Process form
}
```

### Running Specific Tests
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests in specific file
cargo test -p rustgpt --lib module_name::
```

## Database Migration Guide

### Creating New Migration
```bash
# Create migration file
touch db/migrations/$(date +%Y%m%d%H%M%S)_your_migration_name.sql

# Write SQL
sqlx migrate add -r -p sqlite your_migration_name

# Run migrations
just db-migrate
```

### Checking Migration Status
```bash
# View applied migrations
sqlx migrate info

# View pending migrations
sqlx migrate pending
```

## Performance Considerations

### BCrypt Hashing Cost
- **Cost 10**: ~40ms (fast, less secure)
- **Cost 12**: ~100ms (default, balanced)
- **Cost 14**: ~300ms (secure, slow)

Current setting: **Cost 12** (in `src/utils/mod.rs`)

### Optimization Tips
- Database indexes on frequently queried fields ✅
- Validation early in handler to fail fast ✅
- Consider caching for read-heavy operations

## Security Checklist for PRs

Before submitting a pull request, ensure:
- [ ] All user inputs are validated
- [ ] Sensitive operations check authentication
- [ ] Passwords use `hash_password()` utility
- [ ] Password verification uses `verify_password()` utility
- [ ] Error messages don't leak sensitive information
- [ ] HTTP status codes are appropriate
- [ ] SQL queries use parameterized queries (SQLx does this)
- [ ] No hardcoded credentials in code
- [ ] New database changes have migration files

## Debugging Tips

### View Application Logs
```bash
RUST_LOG=debug cargo run
```

### Check Database State
```bash
# Connect to database
sqlite3 db/db.db

# View users table
SELECT id, email, SUBSTR(password,1,10) as pwd, created_at FROM users;

# View specific user
SELECT * FROM users WHERE email='test@example.com';

# Check settings
SELECT user_id, SUBSTR(openai_api_key,1,10) as key FROM settings;
```

### Test Password Hashing
```bash
# In Rust REPL or test
use rustgpt::utils::{hash_password, verify_password};

let hash = hash_password("test123").unwrap();
let valid = verify_password("test123", &hash).unwrap();
assert!(valid);
```

### Check Form Validation
```bash
# Test with curl
curl -X POST http://localhost:3000/signup \
  -d "email=&password=short&password_confirmation=short"
# Should return validation error
```

## Common Issues and Solutions

### "Rust not found" during development
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Database locked error
```bash
# Remove stale database locks
rm -f db/db.db-wal db/db.db-shm

# Or reset database
just db-reset
```

### Compilation errors with dependencies
```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo build
```

### Password verification failing
```bash
# Check password hash cost is consistent
# Both hash_password and verify_password should use bcrypt::DEFAULT_COST

# Test separately:
cargo test utils::tests
```

## Performance Profiling

### Measure Endpoint Speed
```bash
# Install hyperfine
cargo install hyperfine

# Benchmark endpoint
hyperfine 'curl http://localhost:3000/login'
```

### Profile Application
```bash
# Run with profiler
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release
```

## Documentation

### Update When Adding Features
- Update `IMPROVEMENTS.md` with new features
- Update `VALIDATION_RULES.md` with new validation rules
- Update `SECURITY_MIGRATION.md` if schema changes
- Update README.md if user-facing changes

### Document Complex Logic
```rust
/// Validates email format and uniqueness
/// 
/// # Arguments
/// * `email` - Email address to validate
/// * `pool` - Database connection pool
///
/// # Returns
/// * `Ok(())` if valid and unique
/// * `Err(String)` with error message if invalid
pub async fn validate_email(email: &str, pool: &Pool) -> Result<(), String> {
    // Implementation
}
```

## Release Checklist

Before deploying to production:
- [ ] All tests pass: `cargo test`
- [ ] Code compiles: `cargo build --release`
- [ ] Database migrations tested: `just db-reset && just db-migrate`
- [ ] Documentation updated
- [ ] Breaking changes documented
- [ ] Migration guide provided if needed
- [ ] Performance acceptable
- [ ] Security audit completed
- [ ] Error messages don't leak info
- [ ] All endpoints tested manually

## Getting Help

### For Questions About
- **Code structure**: Check README.md and CLAUDE.md
- **Security**: Check SECURITY_MIGRATION.md and VALIDATION_RULES.md
- **Validation rules**: Check VALIDATION_RULES.md
- **Improvements made**: Check IMPROVEMENTS.md
- **Database schema**: Check db/migrations/

### For Debugging
1. Check application logs
2. Check database state
3. Test endpoint with curl or Postman
4. Review recent code changes (git diff)
5. Check error messages in response body

---

**Last Updated**: December 2024
**Status**: Ready for development and testing
