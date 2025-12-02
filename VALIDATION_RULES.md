# RustGPT Input Validation Rules

This document provides a quick reference for all input validation rules implemented in RustGPT.

## Authentication

### Login Form
| Field | Rule | Error |
|-------|------|-------|
| Email | Non-empty, must contain '@' | Invalid credentials |
| Password | Non-empty | Invalid credentials |

### Signup Form
| Field | Rule | Error |
|-------|------|-------|
| Email | Non-empty, contains '@', max 255 chars | Validation failed |
| Password | Min 8 chars, max 255 chars | Weak password |
| Confirm Password | Must match password | Password mismatch |

### Password Change
| Field | Rule | Error |
|-------|------|-------|
| Current Password | Must match stored hash | Invalid password |
| New Password | Min 8 characters | Weak password |
| Confirm Password | Must match new password | Password mismatch |

## Chat Operations

### Create New Chat
| Field | Rule | Error |
|-------|------|-------|
| Message | Non-empty after trim, max 10,000 chars | Invalid chat |
| Model | Non-empty, must be selected | Invalid chat |

### Add Message to Chat
| Field | Rule | Error |
|-------|------|-------|
| Message | Non-empty after trim, max 10,000 chars | Invalid message |

## Settings

### API Key Update
| Field | Rule | Error |
|-------|------|-------|
| API Key | Non-empty, min 20 chars | Invalid API key |
| User | Must be authenticated | Unauthorized |

## Provider Configuration

### Create Provider
| Field | Rule | Error |
|-------|------|-------|
| Name | Non-empty, max 100 chars | Provider name invalid |
| Provider Type | Must be valid enum | Type validation |
| Base URL | Must start with http:// or https:// | URL format error |
| API Key | Non-empty, min 10 chars | API key too short |

## Agent Configuration

### Create Agent
| Field | Rule | Error |
|-------|------|-------|
| Name | Non-empty, max 100 chars | Name invalid |
| Description | Optional, no specific limit | N/A |
| Provider ID | Must be > 0 | Invalid provider |
| Model Name | Non-empty | Model required |
| Stream | Optional boolean | N/A |
| Chat | Optional boolean | N/A |
| Embed | Optional boolean | N/A |
| Image | Optional boolean | N/A |
| Tool | Optional boolean | N/A |
| Tools | Optional string array | N/A |
| Allow Tools | Optional string array | N/A |
| System Prompt | Optional string | N/A |
| Top P | Optional, 0.0 ≤ value ≤ 1.0 | Invalid probability |
| Max Context | Optional, 1 ≤ value ≤ 1,000,000 | Context too large |
| File | Optional boolean | N/A |
| File Types | Optional string array | N/A |
| Temperature | Optional, 0.0 ≤ value ≤ 2.0 | Temperature invalid |
| Max Tokens | Optional, 1 ≤ value ≤ 100,000 | Tokens invalid |
| Presence Penalty | Optional float | N/A |
| Frequency Penalty | Optional float | N/A |
| Icon | Optional emoji string | N/A |
| Category | Optional string | N/A |
| Public | Optional boolean | N/A |

## HTTP Status Codes

| Code | Meaning | Common Triggers |
|------|---------|-----------------|
| 200 | OK | Successful request |
| 201 | Created | Resource created successfully |
| 400 | Bad Request | Validation failure, missing/invalid fields |
| 401 | Unauthorized | Not authenticated or wrong credentials |
| 404 | Not Found | Resource doesn't exist |
| 500 | Server Error | Database error, system failure |

## Validation Patterns

### Email Validation
```
- Must be non-empty
- Must contain exactly one '@' character
- For signup: max 255 characters
```

### Password Validation
```
- Must be non-empty for login
- For signup: min 8 characters, max 255 characters
- Hashed using BCrypt (cost: 12)
- Compared using constant-time verification
```

### URL Validation
```
- Must be non-empty
- Must start with http:// or https://
- Full URL format validation (reserved for future enhancement)
```

### Numeric Range Validation
```
- Temperature: [0.0, 2.0]
- Top-p: [0.0, 1.0]
- Max Tokens: [1, 100000]
- Max Context: [1, 1000000]
```

### String Length Validation
```
- Provider/Agent Name: max 100 characters
- Email: max 255 characters
- Password: 8-255 characters
- Chat Message: max 10,000 characters
- API Key: min 10 characters
```

## Security Considerations

### Password Security
- Passwords are never stored in plain text
- BCrypt hashing with configurable cost (default: 12)
- Verification uses constant-time comparison
- Passwords cannot be retrieved (one-way hash)

### API Keys
- Validated for minimum length
- User must be authenticated to update
- Stored in database (encryption recommended for production)

### Input Sanitization
- Length limits prevent buffer overflow attacks
- Format validation prevents injection attacks
- Type validation ensures data consistency

### Authentication
- Session-based using HTTP-only cookies
- Session tokens prevent CSRF attacks (cookies HTTP-only)
- Failed authentication doesn't reveal user existence

## Future Enhancements

### Planned Validations
- Email format validation (RFC 5322)
- Strong password requirements (uppercase, lowercase, numbers, symbols)
- Rate limiting on login attempts
- Account lockout after failed attempts
- CAPTCHA for repeated failures
- IP-based access control

### Recommended for Production
- Enable HTTPS only
- Implement CSRF tokens on forms
- Add Content Security Policy headers
- Implement API key encryption at rest
- Enable audit logging for all validation failures
