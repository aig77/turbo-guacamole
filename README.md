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
- Redis caching for faster reads

## Endpoints

**Public Routes:**
- `GET /{code}` - Redirect to original URL
- `POST /shorten` - Create shortened URL (body: `{"url": "https://example.com"}`)

**Analytics Routes:**
- `GET /{code}/stats` - Total and daily clicks 

**Admin Routes (Basic Auth):**
- `GET /admin/codes` - List all URL mappings
- `DELETE /admin/codes` - Delete all URLs
- `DELETE /admin/codes/{code}` - Delete specific URL

**Other:**
- `GET /health` - Verifies application health by checking database connections

## Development
This project utilizes Postgres and Redis. For local development, ensure you have docker and docker-compose installed.

```bash
# Start containers
docker-compose up -d

# Stop containers:
docker-compose down

# Remove volumes and stop
docker-compose down -v
```

You can also install cli tools to interact with the databases. `psql` and `redis-cli` are included in this project's nix shell.

_Postgres_
```bash
psql -h localhost -p 5432 -U postgres -d postgres
# or
docker exec -it postgres psql -U postgres
```
_Redis_
```bash
redis-cli -h localhost -p 6379
# or
docker exec -it redis redis-cli
```

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
- [x] Redirect caching | _redis implemented_
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
