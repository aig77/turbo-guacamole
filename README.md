# Turbo Guacamole 🥑 

A simple URL Shortener in Rust.

## Basic Features
- Random 6-character Base62 code generation
- Collision handling with automatic retry
- Duplicate URL detection
- PostgreSQL persistence
- Request logging and tracing with request IDs
- Click analytics
- Redis caching for faster reads
- Automatically delete stale URLs
- Swagger UI

## Endpoints

**Main Routes:**
- `GET /{code}` - Redirect to original URL
- `POST /shorten` - Create shortened URL (body: `{"url": "https://example.com"}`)

**Analytics:**
- `GET /stats` - Total URLs and clicks
- `GET /{code}/stats` - Total and daily clicks by code

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
- [x] Postgres Migration | _sqlx + postgres_
- [x] Collision strategy | _change to random code generation and handle collision using retries_ 
- [x] Max collision retries | _set to 5_
- [x] Logging | _tokio tracing_
- [x] Modular Structure | _great example [here](https://rust-api.dev/docs/part-1/tokio-hyper-axum/#routing)_
- [x] Analytics endpoints | _click table tracks redirects + endpoint retrieves total and daily clicks for a single code_
- [x] Rate limit | _distinct ip rate limits on code and shorten endpoints_
- [x] Graceful shutdown | _copied [axum example](https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs)_
- [x] Url length limit | _2048 should be long enough_
- [x] Health check endpoint | _checks database connection_
- [x] Redirect caching | _redis implemented_
- [x] Clear stale URLs | _added async task to clear daily (exceeding 90 days without clicks)_
- [x] Custom api error type
- [x] Request ID / correlation header
- [x] OpenAPI spec
- [x] Frontend
- [ ] App dockerfile
- [ ] CI Pipeline

# Future Considerations
- JWT
