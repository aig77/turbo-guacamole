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
- `GET /{code}/stats` - Total and daily clicks 
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
- [x] Modular Structure | _great example [here](https://rust-api.dev/docs/part-1/tokio-hyper-axum/#routing)_
- [x] Analytics | _click table tracks redirects_
- [x] Analytics endpoints | _total and daily clicks for a single code_
- [x] Rate limit | _distinct ip rate limits on code and shorten endpoints_
- [x] Graceful shutdown | _copied [axum example](https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs)_
- [x] Url length limit | _2048 should be long enough_
- [x] Health check endpoint | _checks database connection_
- [ ] Redirect caching
- [ ] URL TTL
- [ ] JWT
- [ ] CORS Configuration
- [ ] Custom api error type
- [ ] Request ID / correlation header
- [ ] App dockerfile
- [ ] OpenAPI spec
- [ ] CI Pipeline

# Future Considerations
- Frontend
