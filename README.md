# deckwatch-plugin-redis

A [deckwatch](https://github.com/alexconrey/deckwatch) plugin that provisions a Redis cache
sidecar for an application via the **Infrastructure tab** — no annotations required.

## Usage

1. Open the Application detail page in deckwatch.
2. Click the **Infrastructure** tab.
3. Click **+ Add Redis Cache**.
4. Fill in any optional fields (image, port) and click **Provision**.

Once provisioned, deckwatch injects on the next reconcile:

- **Sidecar**: `redis:7-alpine` on port 6379 (50m CPU / 64Mi memory), named `redis-sidecar`
- **Env var**: `REDIS_URL=redis://localhost:{port}` into every deployment in the application
- **Annotation**: `deckwatch.io/redis: "provisioned"` on all deployments

To remove the sidecar, click **Deprovision** on the Infrastructure tab. The sidecar is removed
from all deployments on the next reconcile via server-side apply.

## Configuration

### Resource fields (per-provisioned instance)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `REDIS_IMAGE` | String | `redis:7-alpine` | Redis container image |
| `REDIS_PORT` | String | `6379` | Port the Redis sidecar listens on |

### Plugin config (global defaults)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `REDIS_IMAGE` | String | `redis:7-alpine` | Default Redis image (overridable per-resource) |

Set global defaults in **Settings → Plugins → Redis → Configure**.

## Building

```bash
# Install the wasm32 target (once)
rustup target add wasm32-unknown-unknown

# Build the WASM artifact
make build          # → dist/plugin.wasm

# Run host-target unit tests
make test
```

## Marketplace

This plugin is available on the [deckwatch marketplace](http://market.deckwatch.io).

To install via the deckwatch UI: **Settings → Plugins → Browse Marketplace → Redis**.

### Submitting to the marketplace

To add or update this plugin in the marketplace catalog, open a PR against
[alexconrey/deckwatch-marketplace](https://github.com/alexconrey/deckwatch-marketplace)
adding the following entry to `catalog.json`:

```json
{
  "name": "Redis",
  "slug": "deckwatch-plugin-redis",
  "description": "Provisions a Redis cache sidecar via the Application Infrastructure tab.",
  "author": "alexconrey",
  "homepage": "https://github.com/alexconrey/deckwatch-plugin-redis",
  "trust_level": "community",
  "tags": ["redis", "cache", "sidecar"],
  "latest_version": "0.1.0",
  "source": {
    "type": "github",
    "repo": "alexconrey/deckwatch-plugin-redis",
    "ref": "v0.1.0",
    "path": "plugin.wasm",
    "use_release": true
  },
  "allowed_hosts_hint": []
}
```

Also add `plugins/deckwatch-plugin-redis.md` following the template in the marketplace repo.

## License

Apache-2.0
