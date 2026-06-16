# Production Deployment

PeliSearch does not implement TLS directly. All encryption, termination, and advanced traffic management should be handled by a **reverse proxy** placed in front of the server.

This guide covers deployment with three popular proxies — nginx, Traefik, and Caddy.

---

## Architecture

```
Client (HTTPS)
     │
     ▼
  Reverse Proxy  (TLS termination, rate limiting, header rewriting)
     │
     ▼
 PeliSearch      (HTTP on 127.0.0.1:7700, no TLS)
```

PeliSearch binds to `127.0.0.1` by default, which is secure behind a reverse proxy. Never expose port 7700 directly to the internet.

---

## Recommended Server Configuration

Create a config file (`pelisearch.toml`):

```toml
host = "127.0.0.1"
port = 7700
data_path = "/var/lib/pelisearch/data"
log_level = "info"
auth_enabled = true
api_key = "your-secret-api-key"
rate_limit_enabled = true
rate_limit_requests_per_minute = 60
```

Or pass flags directly:

```bash
./pelisearch-server \
  --host 127.0.0.1 \
  --port 7700 \
  --data-path /var/lib/pelisearch/data \
  --api-key "your-secret-api-key" \
  --auth-enabled true \
  --rate-limit-enabled true
```

---

## nginx

### Installation

```bash
sudo apt install nginx   # Debian / Ubuntu
sudo dnf install nginx   # Fedora / RHEL
```

### Site Configuration

Create `/etc/nginx/sites-available/pelisearch`:

```nginx
upstream pelisearch {
    server 127.0.0.1:7700;
    keepalive 64;
}

server {
    listen 443 ssl;
    server_name search.example.com;

    ssl_certificate     /etc/ssl/certs/example.com.pem;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    # Maximum request body size (match PeliSearch's `max-body-size` or increase)
    client_max_body_size 10m;

    location / {
        proxy_pass http://pelisearch;

        proxy_set_header Host                  $host;
        proxy_set_header X-Real-IP             $remote_addr;
        proxy_set_header X-Forwarded-For       $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto     $scheme;

        # Pass the API key header from the client
        proxy_set_header Authorization         $http_authorization;
        proxy_set_header X-Api-Key             $http_x_api_key;

        proxy_http_version 1.1;
        proxy_set_header Connection "";
    }

    # Optionally strip the Swagger UI /docs and /openapi.json
    # behind auth or remove them in production
}

server {
    listen 80;
    server_name search.example.com;
    return 301 https://$server_name$request_uri;
}
```

### Health Checks (optional)

If your load balancer needs a health check endpoint:

```nginx
location /health {
    proxy_pass http://pelisearch;
    proxy_http_version 1.1;
}
```

The `/health` and `/ready` endpoints do not require authentication.

---

## Traefik

### Dynamic Configuration (File Provider)

Save as `pelisearch.yml`:

```yaml
http:
  routers:
    pelisearch:
      rule: "Host(`search.example.com`)"
      service: pelisearch
      entryPoints:
        - websecure
      tls:
        certResolver: letsencrypt

  services:
    pelisearch:
      loadBalancer:
        servers:
          - url: "http://127.0.0.1:7700"
        healthCheck:
          path: /health
          interval: 30s
          timeout: 3s
        passHostHeader: true
```

### Docker Compose

```yaml
services:
  traefik:
    image: traefik:v3.0
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - ./traefik.yml:/etc/traefik/traefik.yml
      - ./pelisearch.yml:/etc/traefik/dynamic/pelisearch.yml

  pelisearch:
    build: .
    command:
      - --host
      - "0.0.0.0"
      - --port
      - "7700"
      - --data-path
      - /data
      - --api-key
      - "${PELISEARCH_API_KEY}"
      - --auth-enabled
      - "true"
    ports:
      - "127.0.0.1:7700:7700"
    volumes:
      - pelisearch-data:/data

volumes:
  pelisearch-data:
```

> Note: When using Docker, set `--host 0.0.0.0` so the server listens inside the container, but only expose the port on `127.0.0.1` to keep it local to the host.

---

## Caddy

### Caddyfile

```
search.example.com {
    reverse_proxy 127.0.0.1:7700 {
        # Forward the original client IP
        header_up X-Real-IP {remote_host}
        header_up X-Forwarded-For {remote_host}
        header_up X-Forwarded-Proto {scheme}

        # Pass auth headers
        header_up Authorization {http.request.header.Authorization}
        header_up X-Api-Key {http.request.header.X-Api-Key}
    }

    # Caddy automatically provisions and renews TLS via Let's Encrypt
}
```

### systemd Service for PeliSearch

```ini
[Unit]
Description=PeliSearch server
After=network.target

[Service]
Type=simple
User=pelisearch
Group=pelisearch
ExecStart=/usr/local/bin/pelisearch-server \
    --host 127.0.0.1 \
    --port 7700 \
    --data-path /var/lib/pelisearch/data \
    --api-key ${PELISEARCH_API_KEY}
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

---

## Rate Limiting Behind a Proxy

The built-in rate limiter uses the direct TCP connection IP. When PeliSearch runs behind a reverse proxy, **every client will appear as the proxy's IP**.

For accurate per-client rate limiting in production, use the proxy's own rate limiting features:

| Proxy  | Directive               |
|--------|--------------------------|
| nginx  | `limit_req_zone`         |
| Traefik| `RateLimit` middleware    |
| Caddy  | `rate_limit` global option|

Then disable PeliSearch's built-in rate limiter (`--rate-limit-enabled false`).

---

## Known Limitations

| Concern | Status | Workaround |
|---------|--------|------------|
| TLS termination | Not in PeliSearch | Use nginx / Traefik / Caddy |
| Client IP detection | Rate limiter sees proxy IP | Use proxy's rate limiter instead |
| HTTP/2 | Not configured | Proxy handles HTTP/2 → HTTP/1.1 conversion |
| Request body size | Configurable via `--max-body-size` | Also set `client_max_body_size` in nginx |
| Websockets | Not used | No action needed |

---

## Security Checklist

- [ ] PeliSearch binds to `127.0.0.1` (not `0.0.0.0`)
- [ ] TLS certificate configured and auto-renewing
- [ ] API key set and `auth_enabled = true`
- [ ] Rate limiting enabled on the proxy
- [ ] Firewall blocks port 7700 from external access
- [ ] Data directory permissions restricted (`chown pelisearch:pelisearch`)
- [ ] Systemd `Restart=on-failure` configured
- [ ] Logs are rotated (use `log_level = "warn"` in high-volume production)
