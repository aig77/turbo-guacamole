# Turbo Guacamole 🥑 

A simple URL Shortener in Rust.

## Basic Features

**Functional**
- Shorten URLs to 6-character Base62 codes
- Collision retry + duplicate URL detection
- URL validation (http/https, 2048 char limit)
- Redirect with Redis read-through caching
- Click analytics — global + per-code totals, daily breakdown
- Automatic stale-URL cleanup
- Health checks and OpenAPI spec
- Static frontend at `/`

**Non-Functional**
- Performance — Redis caching (1h TTL), connection pooling (bb8/sqlx), cached global stats
- Reliability — graceful shutdown, bounded collision retries, DB-first failover on cache miss
- Scalability — stateless app; external Postgres/Redis supported (NixOS `createDatabases = false`)
- Security — per-endpoint IP rate limiting, strict URL scheme validation
- Observability — structured request logging with `X-Request-ID` and per-request latency
- Deployability — Dockerfile, `nixosModules.default`, Fly.io guide
- Testability — NixOS integration test (full boot + reboot) gated to x86_64-linux

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
  inputs.turbo-guacamole.url = "github:aig77/turbo-guacamole";

  outputs = { nixpkgs, turbo-guacamole, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        turbo-guacamole.nixosModules.default
        {
          services.turbo-guacamole = {
            enable = true;
            host = "0.0.0.0";
            port = 8080;
          };
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
| `services.turbo-guacamole.createDatabases` | `true` | Bundle PostgreSQL + Redis on the machine |
| `services.turbo-guacamole.environmentFile` | `null` | Optional `KEY=VALUE` file; overrides module-set values (e.g. `/run/secrets/tg.env`) |

With `createDatabases = true` (the default) the module sets up Postgres, Redis, and a one-shot `turbo-guacamole-schema` service that applies `sql/schema.sql` before the app starts.

To use external databases, disable the bundled ones and point the service at them via an `environmentFile`:

```nix
services.turbo-guacamole = {
  enable = true;
  createDatabases = false;
  environmentFile = "/run/secrets/tg.env";
};
```

The file must be readable by the `turbo-guacamole` user at service start. When `createDatabases = false` it **must** define `DATABASE_URL` and `CACHE_URL`, since the module doesn't set them itself. Values from the file always override what the module sets via `Environment=` (systemd `EnvironmentFile=` takes precedence).

## Deployment

See [DEPLOYMENT.md](./docs/DEPLOYMENT.md) for a complete guide on deploying to production with Fly.io + Hetzner VPS.
