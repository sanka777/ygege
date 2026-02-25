---
sidebar_position: 5
sidebar_label: Configuration
---

# Configuration

This guide details all available configuration options for Ygégé.

## config.json File

The main configuration file is `config.json`. It should be placed in the `/config` folder (Docker) or at the project root (manual installation).

### Complete Structure

```json
{
    "username": "your_ygg_username",
    "password": "your_password",
    "bind_ip": "0.0.0.0",
    "bind_port": 9876,
    "log_level": "debug",
    "tmdb_token": null,
    "ygg_domain": null,
    "turbo_enabled": null,
    "anti_bot_provider": "flaresolverr",
    "flaresolverr_url": "http://flaresolverr:8191"
}
```

## Available Options

### YGG Authentication

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `username` | string | ✅ | YGG Torrent username |
| `password` | string | ✅ | YGG Torrent password |

:::warning Warning
Without valid credentials, you will be **rate-limited** by YGG and the service won't work properly.
:::

### Network Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `bind_ip` | string | `0.0.0.0` | Listening IP address |
| `bind_port` | number | `9876` | Server listening port |

:::tip Custom Port
To avoid port conflicts (e.g., on Windows), simply change `BIND_PORT`:
```yaml
environment:
  BIND_PORT: "3000"  # Use port 3000 instead of 9876
ports:
  - "3000:3000"
```
The healthcheck automatically adapts using `$${BIND_PORT:-9876}`.
:::

### Logging

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `log_level` | string | `info` | Log verbosity level |

Available levels:
- `trace`: Maximum details (development)
- `debug`: Debug information
- `info`: General information
- `warn`: Warnings only
- `error`: Errors only

### TMDB/IMDB Metadata

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `tmdb_token` | string | `null` | TMDB API key (optional) |

### YGG Domain Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `ygg_domain` | string | `null` | Custom YGG domain (optional) |
| `anti_bot_provider` | string | `flaresolverr` | Anti-bot provider (`flaresolverr` or `native`) |
| `flaresolverr_url` | string | `http://flaresolverr:8191` | Flaresolverr service URL |

:::tip When to use YGG_DOMAIN?
By default, Ygégé automatically detects the current YGG domain via a redirect from `ygg.re`. If this auto-detection fails (307 errors, "Session expired..."), you can manually specify the domain:
```
YGG_DOMAIN=www.yggtorrent.org
```
:::

## Environment Variables

All options can also be set via environment variables:

| Variable | config.json equivalent |
|----------|------------------------|
| `YGG_USERNAME` | `username` |
| `YGG_PASSWORD` | `password` |
| `BIND_IP` | `bind_ip` |
| `BIND_PORT` | `bind_port` |
| `LOG_LEVEL` | `log_level` |
| `TMDB_TOKEN` | `tmdb_token` |
| `YGG_DOMAIN` | `ygg_domain` |
| `TURBO_ENABLED` | `turbo_enabled` |
| `ANTI_BOT_PROVIDER` | `anti_bot_provider` |
| `FLARESOLVERR_URL` | `flaresolverr_url` |

:::tip Priority
Environment variables have **priority** over config.json file.
:::

## Complete Configuration Example

### For Docker Compose

```yaml
services:
  flaresolverr:
    image: ghcr.io/flaresolverr/flaresolverr:latest
    container_name: flaresolverr
    restart: unless-stopped
    ports:
      - "8191:8191"
    healthcheck:
      test: ["CMD-SHELL", "curl --fail http://localhost:8191/ || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 5

  ygege:
    image: uwucode/ygege:latest
    container_name: ygege
    restart: unless-stopped
    depends_on:
      flaresolverr:
        condition: service_healthy
    ports:
      - "9876:9876"
    volumes:
      - ./config:/config
    environment:
      YGG_USERNAME: "my_username"
      YGG_PASSWORD: "my_password"
      LOG_LEVEL: "info"
      TMDB_TOKEN: "your_tmdb_token"
      ANTI_BOT_PROVIDER: "flaresolverr"
      FLARESOLVERR_URL: "http://flaresolverr:8191"
      # YGG_DOMAIN: "www.yggtorrent.org"  # Optional: force a specific domain
      # TURBO_ENABLED: true  # Optional : If turbo is enabled, no need to wait 30s between token and torrent download
```

### For config.json File

```json
{
    "username": "my_username",
    "password": "my_password",
    "bind_ip": "0.0.0.0",
    "bind_port": 9876,
    "log_level": "debug",
    "tmdb_token": "your_tmdb_token",
    "ygg_domain": null,
    "turbo_enabled": true,
    "anti_bot_provider": "flaresolverr",
    "flaresolverr_url": "http://flaresolverr:8191"
}
```


## Flaresolverr Troubleshooting

### Cannot connect to Flaresolverr
- Check the service is running: `curl http://localhost:8191/`
- Check `FLARESOLVERR_URL` points to the correct endpoint (`http://flaresolverr:8191` in Docker Compose).
- Ensure both services are on the same Docker network.

### Ygégé starts but searches fail
- Check Ygégé logs: `docker logs ygege`
- Check Flaresolverr logs: `docker logs flaresolverr`
- As a fallback, test native mode: `ANTI_BOT_PROVIDER=native`

### Timeouts or intermittent Cloudflare responses
- Restart Flaresolverr (`docker restart flaresolverr`).
- Reduce parallel requests in clients (Prowlarr/Jackett).
- Ensure host and container system time are correct.

## Configuration Validation

To verify your configuration is correct, check the logs at startup:

```bash
docker logs ygege
```

You should see:
```
[INFO] Configuration loaded successfully
[INFO] Connecting to YGG Torrent...
[INFO] Authentication successful
[INFO] Server started on 0.0.0.0:9876
```

## Next Steps

- [API Documentation](./api)
- [Prowlarr Integration](./integrations/prowlarr)
