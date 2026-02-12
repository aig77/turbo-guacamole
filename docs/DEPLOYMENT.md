# Deployment Guide

This guide covers deploying the URL shortener with the app on Fly.io and databases on a Hetzner VPS using Dokploy.

## Architecture

- **App**: Fly.io (auto-scaling, edge locations)
- **Databases**: Hetzner VPS with Dokploy (PostgreSQL + Redis)
- **Domain**: Cloudflare DNS
- **CI/CD**: GitHub Actions

---

## Part 1: Database Setup on Hetzner VPS

### 1.1 Install Dokploy on Hetzner VPS

SSH into your Hetzner VPS and install Dokploy:

```bash
curl -sSL https://dokploy.com/install.sh | sh
```

Access Dokploy at `http://your-vps-ip:3000`

### 1.2 Deploy PostgreSQL

1. In Dokploy, create a new **Compose** project
2. Name it `url-shortener-db`
3. Use this docker-compose.yml:

```yaml
services:
  postgres:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: urlshortener
    ports:
      - "5432:5432"
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    command: redis-server --requirepass ${REDIS_PASSWORD} --appendonly yes --maxmemory 256mb --maxmemory-policy allkeys-lru
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "--pass", "${REDIS_PASSWORD}", "ping"]
      interval: 10s
      timeout: 3s
      retries: 3

volumes:
  postgres-data:
  redis-data:
```

4. Set environment variables in Dokploy:
   - `POSTGRES_PASSWORD`: Strong password for PostgreSQL
   - `REDIS_PASSWORD`: Strong password for Redis

5. Deploy the stack

### 1.3 Initialize Database Schema

SSH into your VPS and run:

```bash
# Get the Postgres container ID
docker ps | grep postgres

# Copy schema file (or paste it directly)
docker exec -i <postgres-container-id> psql -U postgres -d urlshortener <<EOF
CREATE TABLE IF NOT EXISTS urls (
  code VARCHAR(6) PRIMARY KEY,
  url TEXT NOT NULL UNIQUE,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS clicks (
  id BIGSERIAL PRIMARY KEY,
  code VARCHAR(6) REFERENCES urls(code) ON DELETE CASCADE,
  clicked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_clicks_code_date ON clicks(code, clicked_at);
EOF
```

### 1.4 Secure Database Access

**Option A: Expose Publicly with Firewall (Simpler)**

Configure UFW to allow only Fly.io IP ranges:

```bash
# Get Fly.io IP ranges from: https://fly.io/docs/reference/regions/
# Example for North America regions:
ufw allow from 66.241.124.0/24 to any port 5432
ufw allow from 66.241.125.0/24 to any port 5432
ufw allow from 66.241.124.0/24 to any port 6379
ufw allow from 66.241.125.0/24 to any port 6379

# Enable firewall
ufw enable
```

**Option B: Fly.io WireGuard VPN (More Secure)**

1. On your VPS, install WireGuard:
```bash
apt update && apt install wireguard
```

2. Set up Fly.io WireGuard peering (requires Fly.io config)
3. Keep PostgreSQL/Redis on private network only

**Recommendation**: Start with Option A, switch to Option B for production.

---

## Part 2: Fly.io Setup

### 2.1 Install Flyctl

```bash
curl -L https://fly.io/install.sh | sh
```

### 2.2 Login and Create App

```bash
fly auth login
fly apps create turbo-guacamole
```

### 2.3 Set Secrets

Replace with your actual VPS IP and passwords:

```bash
fly secrets set \
  DATABASE_URL="postgres://postgres:YOUR_POSTGRES_PASSWORD@YOUR_VPS_IP:5432/urlshortener" \
  CACHE_URL="redis://:YOUR_REDIS_PASSWORD@YOUR_VPS_IP:6379"
```

### 2.4 Choose Region

```bash
# List regions
fly regions list

# Set primary region (close to your VPS for lower latency)
# If VPS is in Germany, use Frankfurt (fra)
fly regions set fra
```

### 2.5 Deploy

```bash
fly deploy
```

### 2.6 Verify Deployment

```bash
fly status
fly logs

# Test health endpoint
curl https://turbo-guacamole.fly.dev/health
```

---

## Part 3: Custom Domain with Cloudflare

### 3.1 Get Fly.io IP Address

```bash
fly ips list
```

You'll get an IPv4 and IPv6 address.

### 3.2 Configure Cloudflare DNS

1. Log into Cloudflare
2. Select your domain
3. Go to DNS → Records
4. Add records:

**For root domain (example.com):**
- Type: `A`, Name: `@`, Content: `<fly-ipv4>`, Proxy: Off
- Type: `AAAA`, Name: `@`, Content: `<fly-ipv6>`, Proxy: Off

**For subdomain (short.example.com):**
- Type: `A`, Name: `short`, Content: `<fly-ipv4>`, Proxy: Off
- Type: `AAAA`, Name: `short`, Content: `<fly-ipv6>`, Proxy: Off

**Important**: Set Proxy to "Off" initially for SSL setup.

### 3.3 Add Certificate to Fly.io

```bash
# For root domain
fly certs add example.com

# For subdomain
fly certs add short.example.com

# Check status
fly certs show example.com
```

Wait for DNS to propagate and certificate to issue (5-30 minutes).

### 3.4 Enable Cloudflare Proxy (Optional)

Once SSL is working:
1. Return to Cloudflare DNS
2. Enable "Proxied" (orange cloud) for your records
3. This gives you Cloudflare's DDoS protection + caching

### 3.5 Update Fly.io Config

If using a specific domain, you may want to update `fly.toml`:

```toml
[http_service]
  force_https = true
  [[http_service.checks]]
    interval = "30s"
    timeout = "5s"
    method = "GET"
    path = "/health"
```

---

## Part 4: CI/CD Setup

### 4.1 Generate Fly.io Deploy Token

```bash
fly tokens create deploy -x 999999h
```

Copy the token output.

### 4.2 Add GitHub Secret

1. Go to your GitHub repo
2. Settings → Secrets and variables → Actions
3. Click "New repository secret"
4. Name: `FLY_API_TOKEN`
5. Value: Paste the token
6. Save

### 4.3 Test Auto-Deploy

Push to main branch:

```bash
git add .
git commit -m "Set up deployment"
git push origin main
```

Watch the deployment in GitHub Actions tab.

---

## Part 5: Maintenance & Monitoring

### 5.1 View Logs

```bash
# Fly.io app logs
fly logs

# Database logs (on VPS)
docker logs <postgres-container-id>
docker logs <redis-container-id>
```

### 5.2 Scale App

```bash
# Add more machines
fly scale count 2

# Scale VM resources
fly scale vm shared-cpu-2x --memory 1024
```

### 5.3 Database Backups

On your Hetzner VPS, set up automated PostgreSQL backups:

```bash
# Create backup script
cat > /root/backup-postgres.sh <<'EOF'
#!/bin/bash
BACKUP_DIR="/root/backups"
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p $BACKUP_DIR

docker exec <postgres-container-id> pg_dump -U postgres urlshortener | gzip > $BACKUP_DIR/urlshortener_$DATE.sql.gz

# Keep only last 7 days
find $BACKUP_DIR -name "urlshortener_*.sql.gz" -mtime +7 -delete
EOF

chmod +x /root/backup-postgres.sh

# Add to crontab (daily at 2 AM)
crontab -e
# Add: 0 2 * * * /root/backup-postgres.sh
```

### 5.4 Monitor Health

```bash
# Check app health
fly status
fly checks list

# Check database connectivity from Fly.io
fly ssh console
nc -zv YOUR_VPS_IP 5432
nc -zv YOUR_VPS_IP 6379
```

---

## Troubleshooting

### App can't connect to database

1. Check firewall rules on VPS:
```bash
ufw status
```

2. Verify database is listening on 0.0.0.0:
```bash
docker ps
docker logs <postgres-container-id>
```

3. Test from Fly.io app:
```bash
fly ssh console
nc -zv YOUR_VPS_IP 5432
```

### SSL Certificate Issues

```bash
fly certs show example.com
# If stuck, try removing and re-adding:
fly certs remove example.com
fly certs add example.com
```

### High Database Latency

Move Fly.io app to region closer to VPS:
```bash
fly regions set fra  # Frankfurt, if VPS is in Germany
```

---

## Security Checklist

- [ ] Strong passwords for PostgreSQL and Redis
- [ ] Firewall configured to allow only Fly.io IPs
- [ ] Database not exposed to 0.0.0.0 without protection
- [ ] SSL/TLS enabled on Fly.io (force_https = true)
- [ ] Secrets managed via `fly secrets` (not in code)
- [ ] Regular database backups configured
- [ ] Cloudflare proxy enabled (DDoS protection)
- [ ] Rate limiting configured in app

---

## Quick Reference

### Fly.io Commands
```bash
fly deploy                    # Manual deploy
fly logs                      # View logs
fly ssh console              # SSH into app
fly secrets list             # List secrets
fly secrets set KEY=value    # Set secret
fly status                   # App status
fly scale count 2            # Scale to 2 machines
```

### Dokploy (on VPS)
- Access: `http://YOUR_VPS_IP:3000`
- View logs, restart services, manage volumes

### Update Database Connection
```bash
fly secrets set DATABASE_URL="new-connection-string"
fly secrets set CACHE_URL="new-redis-connection-string"
```
