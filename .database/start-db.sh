#!/bin/sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Stop and remove container if it exists
docker stop postgres-dev 2>/dev/null
docker rm postgres-dev 2>/dev/null

# Start fresh container
docker run -d \
  --name postgres-dev \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_DB=urlshortener \
  -p 5432:5432 \
  -v $SCRIPT_DIR/data:/var/lib/postgresql \
  postgres:latest

# Wait for postgres to be ready
sleep 2

# Create table
docker exec -i postgres-dev psql -U postgres -d urlshortener < $SCRIPT_DIR/init.sql
