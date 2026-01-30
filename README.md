# turbo-guacamole

A simple URL Shortener in Rust.

## Basic Features
- Random 6-character Base62 code generation
- Collision handling with automatic retry
- Duplicate URL detection
- Basic authentication for admin routes
- PostgreSQL persistence
- Request logging and tracing
- Click analytics

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
- [x] Modular Structure
- [x] Analytics | _click table tracks redirects_
- [x] Rate limit | _distinct ip rate limits on code and shorten endpoints_
- [x] Graceful shutdown
- [x] Url length limit | _2048 should be good enough_
- [ ] Health check endpoint
- [ ] Analytics endpoints
- [ ] CORS Configuration
- [ ] Custom api error type
- [ ] Request ID / correlation header
- [ ] App dockerfile
- [ ] OpenAPI spec
- [ ] CI Pipeline

## Future considerations:
- Redis Cache 
- TTL Implemenation
- JWT Auth
