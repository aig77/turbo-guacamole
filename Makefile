COMPOSE := docker compose -f docker-compose.yml
PG      := docker exec -i postgres psql -U postgres

.PHONY: help deps-up deps-down deps-reset dev run psql redis schema-reload check vm vm-check

help: ## Show this help
	@grep -E '^[a-zA-Z-]+:.*?## ' $(MAKEFILE_LIST) | sed 's/:.*## /: /' | sort

deps-up: ## Start Postgres and Redis containers
	$(COMPOSE) up -d

deps-down: ## Stop containers
	$(COMPOSE) down

deps-reset: ## Stop containers and wipe volumes (re-applies schema on next up)
	$(COMPOSE) down -v

dev: deps-up ## Start deps and run the app (creates .env on first run)
	@test -f .env || cp .env.example .env
	cargo run

run: ## Run the app (deps must already be up)
	cargo run

psql: ## Open psql against the Postgres container
	docker exec -it postgres psql -U postgres

redis: ## Open redis-cli against the Redis container
	docker exec -it redis redis-cli

schema-reload: ## Re-apply sql/schema.sql to the running database
	$(PG) -v ON_ERROR_STOP=1 < sql/schema.sql

check: ## Format check + clippy
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

vm: ## Run the full NixOS test VM (UI on http://localhost:8080)
	nix run .#vm

vm-check: ## End-to-end integration test in a NixOS VM
	nix flake check