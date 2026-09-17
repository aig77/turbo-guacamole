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
- OpenAPI spec at `/api-docs/openapi.json`

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

Dependencies (Postgres + Redis) run via Docker; the rest of the tooling comes from the nix dev shell.

### Quickstart

```bash
make dev
```

This starts the dependency containers, copies `.env.example` to a gitignored `.env` on first run, and launches the app. Edit `.env` if your databases aren't on the localhost defaults.

### Managing dependencies

```bash
make deps-up      # Start Postgres + Redis containers
make deps-down    # Stop them
make deps-reset   # Stop and wipe volumes (schema re-applies on next up)
```

### Databases

```bash
make psql            # psql into the Postgres container
make redis           # redis-cli into the Redis container
make schema-reload   # Re-apply sql/schema.sql to a running database
```

### Checks

```bash
make check      # cargo fmt --check + clippy
make vm-check   # Headless end-to-end test in a NixOS VM (nix flake check)
```

### Integration test VM

`nix run .#vm` boots a full NixOS VM (Postgres + Redis + the packaged binary) with the UI on http://localhost:8080 — handy as a pre-deploy sanity check.

`psql` and `redis-cli` are also available directly from the nix shell if you prefer not to use the containers:

```bash
psql -h localhost -p 5432 -U postgres -d postgres
redis-cli -h localhost -p 6379
```

## Deploying with NixOS

The flake exports a NixOS module (`nixosModules.default`) that runs turbo-guacamole as a systemd service, with optional bundled Postgres and Redis. This is the recommended way to deploy to a NixOS machine.

```nix
{
  inputs.turbo-guacamole.url = "github:you/turbo-guacamole";

  outputs = { nixpkgs, turbo-guacamole, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        turbo-guacamole.nixosModules.default
        {
          services.turbo-guacamole.enable = true;
          services.turbo-guacamole.host = "0.0.0.0";
          services.turbo-guacamole.port = 8080;
        }
      ];
    };
  };
}
```

### Options

| Option | Default | Description |
| --- | --- | --- |
| `services.turbo-guacamole.enable` | `false` | Enable the service |
| `services.turbo-guacamole.host` | `127.0.0.1` | Bind address |
| `services.turbo-guacamole.port` | `8080` | HTTP port |
| `services.turbo-guacamole.database.local` | `true` | Bundle PostgreSQL + Redis on the machine |
| `services.turbo-guacamole.database.postgresUrl` | `""` | External PostgreSQL URL (required when `local = false`) |
| `services.turbo-guacamole.database.redisUrl` | `""` | External Redis URL (required when `local = false`) |
| `services.turbo-guacamole.environmentFile` | `null` | `KEY=VALUE` file for secrets, e.g. `/run/secrets/tg.env` |

With `database.local = true` (the default) the module sets up Postgres, Redis, and a one-shot `turbo-guacamole-schema` service that applies `sql/schema.sql` before the app starts.

To use external databases, disable local mode and point the service at them:

```nix
services.turbo-guacamole = {
  enable = true;
  database = {
    local = false;
    postgresUrl = "postgresql://tg:secret@db.internal:5432/turbo_guacamole?sslmode=require";
    redisUrl = "redis://redis.internal:6379";
  };
};
```

For credentials that shouldn't live in the Nix store, pass an `environmentFile` containing `DATABASE_URL` and `CACHE_URL` (e.g. a mounted secret). When `database.local = false` you must supply connection URLs either via `database.postgresUrl`/`database.redisUrl` or via `environmentFile`.

## Deployment

See [DEPLOYMENT.md](./docs/DEPLOYMENT.md) for a complete guide on deploying to production with Fly.io + Hetzner VPS.

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
- [x] App dockerfile
- [x] CI Pipeline

# Future Considerations
- JWT
