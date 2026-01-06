# turbo-guacamole

![](.docs/turbo-guacamole.jpg)

---

A simple URL Shortener in Rust.

## Basic Features
- Random 6-character Base62 code generation
- URL validation (HTTP/HTTPS only)
- Collision handling with automatic retry
- Duplicate URL detection
- Basic authentication for admin routes
- PostgreSQL persistence
- Request logging and tracing

## Endpoints

**Public Routes:**
- `GET /{code}` - Redirect to original URL
- `POST /shorten` - Create shortened URL (body: `{"url": "https://example.com"}`)

**Admin Routes (Basic Auth):**
- `GET /admin/codes` - List all URL mappings
- `DELETE /admin/codes` - Delete all URLs
- `DELETE /admin/codes/{code}` - Delete specific URL

## TODOs:
- [x] Admin route protection | _added basic auth_
- [x] Postgres Migration | _sqlx + postgres_
- [x] Collision strategy | _change to random code generation and handle collision using retries_ 
- [x] Logging | _tokio tracing_

## Future considerations:
- Analytics tracking
- TTL Implemenation
- Stronger auth option
- Redis Cache 
