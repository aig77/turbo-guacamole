# Deployment Guide

Deploy the URL shortener with app on Fly.io and databases on Hetzner VPS (Dokploy).

## Architecture

- **App**: Fly.io
- **Databases**: Hetzner VPS + Dokploy (PostgreSQL + Redis)
- **Network**: WireGuard VPN between Fly.io and Hetzner
- **Domain**: Cloudflare DNS
- **CI/CD**: GitHub Actions

---

## Part 1: Database Setup on Hetzner VPS

### 1.1 Install Dokploy

```bash
curl -sSL https://dokploy.com/install.sh | sh
```

Access at `http://your-vps-ip:3000`

### 1.2 Deploy Databases

Create Compose project in Dokploy. Set `POSTGRES_PASSWORD` and `REDIS_PASSWORD` in Dokploy.

### 1.3 Initialize Schema

```bash
docker exec -i $(docker ps -qf "ancestor=postgres:16-alpine") psql -U postgres -d urlshortener
# paste sql/schema.sql
```

---

## Part 2: WireGuard VPN Setup

### 2.1 Create Fly.io WireGuard Peer

```bash
# Install flyctl if needed
curl -L https://fly.io/install.sh | sh

# Login and create WireGuard configuration
fly auth login
fly wireguard create
```

This generates a WireGuard config. Save the output showing:
- Peer public key
- Peer private key
- Fly.io gateway endpoint
- Peer IP address (e.g., `fdaa:X:X:X::2/120`)

### 2.2 Install WireGuard on Hetzner VPS

```bash
apt update && apt install wireguard resolvconf
```

### 2.3 Configure WireGuard on VPS

Create `/etc/wireguard/fly.conf` from deployment machine using `fly wireguard create`.
Copy file into VM using `scp`.

### 2.4 Start WireGuard

```bash
wg-quick up fly
systemctl enable wg-quick@fly
```

Verify connection:
```bash
wg show
ping6 fdaa:X:X::3  # Ping Fly.io gateway
```

### 2.5 Update Firewall Rules

Since databases are bound to WireGuard interface, block public access by setting a firewall rule. UDP 51820.
Database traffic now flows only through WireGuard tunnel.

---

## Part 3: Fly.io App Setup

### 3.1 Create App

```bash
fly apps create turbo-guacamole
fly regions set fra  # Set region close to VPS
```

### 3.2 Set Secrets

Use WireGuard IPv6 address for VPS connection:

```bash
fly secrets set \
  DATABASE_URL="postgres://postgres:YOUR_POSTGRES_PASSWORD@[fdaa:X:X:X::1]:5432/urlshortener" \
  CACHE_URL="redis://:YOUR_REDIS_PASSWORD@[fdaa:X:X:X::1]:6379"
```

Replace `fdaa:X:X:X::1` with your VPS WireGuard IPv6 address.

### 3.3 Deploy

```bash
fly deploy
fly logs  # Verify deployment
```

---

## Part 4: Custom Domain (Optional)

### 4.1 Get Fly.io IPs

```bash
fly ips list
```

### 4.2 Configure DNS in Cloudflare

Add A/AAAA records pointing to Fly.io IPs (Proxy: Off initially).

### 4.3 Add Certificate

```bash
fly certs add your-domain.com
fly certs show your-domain.com  # Wait for issuance
```

Enable Cloudflare proxy after SSL works.

---

## Part 5: CI/CD Setup

### 5.1 Generate Deploy Token

```bash
fly tokens create deploy -x 999999h
```

### 5.2 Add GitHub Secret

Go to repo Settings → Secrets → Actions → New secret:
- Name: `FLY_API_TOKEN`
- Value: Paste token

Push to main branch triggers auto-deploy via GitHub Actions.

---

## Maintenance

### View Logs
```bash
fly logs
docker logs $(docker ps -qf "ancestor=postgres:16-alpine")
```

### Scale App
```bash
fly scale count 2
fly scale vm shared-cpu-2x --memory 1024
```

### Database Backups
```bash
cat > /root/backup-db.sh <<'EOF'
#!/bin/bash
BACKUP_DIR="/root/backups"
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p $BACKUP_DIR
docker exec $(docker ps -qf "ancestor=postgres:16-alpine") pg_dump -U postgres urlshortener | gzip > $BACKUP_DIR/db_$DATE.sql.gz
find $BACKUP_DIR -name "db_*.sql.gz" -mtime +7 -delete
EOF

chmod +x /root/backup-db.sh
echo "0 2 * * * /root/backup-db.sh" | crontab -
```

---

## Troubleshooting

### Can't connect to database
```bash
# Verify WireGuard is up
wg show

# Test connectivity from Fly.io
fly ssh console
ping6 fdaa:X:X:X::1
nc -zv fdaa:X:X:X::1 5432
```

### Certificate issues
```bash
fly certs show your-domain.com
fly certs remove your-domain.com && fly certs add your-domain.com
```

---

## Quick Reference

```bash
# Fly.io
fly deploy
fly logs
fly ssh console
fly secrets set KEY=value
fly status

# WireGuard
wg show
wg-quick up fly
wg-quick down fly
```
