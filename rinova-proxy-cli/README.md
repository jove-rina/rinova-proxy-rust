# rinova-proxy-cli

Proxy subscription → Clash config — CLI tool.

> **v1.0.0** · Author: **Rina**  
> Binary name: `rinova-proxy-cli`

## Install

```bash
cargo install rinova-proxy-cli
```

Build from source:

```bash
cargo build --release -p rinova-proxy-cli
# binary: target/release/rinova-proxy-cli.exe (Windows) or target/release/rinova-proxy-cli (Unix)
```

---

## Usage

### Single-shot conversion

Fetch a subscription and write a Clash config file.

```bash
rinova-proxy-cli -u "https://your-jms-subscription-url"
```

Default output: `./clash-config.yaml` in the current directory.

```bash
rinova-proxy-cli -u "https://..." -o ~/Downloads/my-clash.yaml
rinova-proxy-cli -u "https://..." -o ./configs/clash.yaml
```

### Rule modes

```bash
# Built-in domain rules (default)
rinova-proxy-cli -u "https://..." --rules builtin

# ACL4SSR external RuleSet
rinova-proxy-cli -u "https://..." --rules external
```

| Mode | Description |
|------|-------------|
| `builtin` | Built-in `DOMAIN-SUFFIX` / `GEOIP` rules |
| `external` | `RULE-SET` pointing to ACL4SSR GitHub lists |

### Merge into existing config

Replace `proxies` in an existing Clash config and update stale node references in policy groups. Rules and group structure are preserved.

```bash
rinova-proxy-cli -u "https://..." --merge ~/.config/clash-verge-rev/profiles/current.yaml
```

If a policy group references a node name that no longer exists, it falls back to the first new node (with a warning).

### HTTP subscription service

Run a local HTTP server for Clash Verge Rev remote subscription import.

```bash
rinova-proxy-cli -p 25500 -u "https://..." -i 60
```

| Flag | Description | Default |
|------|-------------|---------|
| `-p`, `--port` | Listen port | required in serve mode |
| `-u`, `--url` | Subscription URL | required |
| `-i`, `--interval` | Auto-refresh interval (minutes) | `60` |
| `--rules` | `builtin` or `external` | `builtin` |

**Verge Rev setup:**

```
Profiles → Import → Remote subscription
URL:      http://127.0.0.1:25500/clash.yaml
Interval: 60
```

**Endpoints** (bind `127.0.0.1` only):

| Method | Path | Response |
|--------|------|----------|
| GET | `/clash.yaml` | Clash YAML config |
| GET | `/health` | JSON health status |
| POST | `/refresh` | Trigger refresh `{ ok, skipped, nodes }` |
| GET | `/refresh` | 405 Method Not Allowed |

**`/health` JSON fields:**

| Field | Description |
|-------|-------------|
| `status` | `"ok"` or `"initializing"` |
| `nodes` | Current node count |
| `updatedAt` | Last refresh time (ISO 8601) or `null` |
| `nextRefreshMin` | Minutes until next auto-refresh |
| `lastError` | Last refresh error message or `null` |

Stop the server with `Ctrl+C`. On Linux/macOS, `SIGTERM` is also handled.

---

## Options reference

```
rinova-proxy-cli [OPTIONS]

Options:
  -u, --url <URL>            Proxy subscription URL
  -o, --output <OUTPUT>      Output file path [default: clash-config.yaml in cwd]
  -p, --port <PORT>          Serve mode: HTTP port
  -i, --interval <MINUTES>   Serve mode: refresh interval [default: 60]
      --rules <MODE>         Rule mode: builtin | external [default: builtin]
      --merge <FILE>         Merge into existing Clash config
  -h, --help                 Print help
  -V, --version              Print version
```

### Modes

| Invocation | Mode |
|------------|------|
| `rinova-proxy-cli -u <url>` | Convert once, write YAML |
| `rinova-proxy-cli -p <port> -u <url>` | HTTP subscription service |
| `rinova-proxy-cli` | Print help |

Serve mode requires `-u`. If `-o` is omitted, output defaults to `./clash-config.yaml`.

---

## Output example

```
🔗 Fetching subscription: https://example.com/sub?token=***
✅ Parsed: 6 nodes
   1. [ss] 1.2.3.4:6191 ← Node-A
   2. [vmess] 5.6.7.8:6191 ← Node-B
   ...

✅ Config written: /path/to/clash-config.yaml
📊 6 nodes, 6 groups
```

Subscription URLs are masked in logs (query parameters replaced with `***`).

---

## Internationalization

Messages and `--help` text follow `LANG` / `LC_ALL`:

```bash
# Chinese help and output
LANG=zh_CN.UTF-8 rinova-proxy-cli --help
LANG=zh_CN.UTF-8 rinova-proxy-cli -u "https://..."

# English (default on most systems)
LANG=en_US.UTF-8 rinova-proxy-cli --help
```

---

## Development

```bash
# From repository root
cargo run -p rinova-proxy-cli -- -u "https://..."
cargo run -p rinova-proxy-cli -- -p 25500 -u "https://..." -i 60
```

---

## License

MIT — Copyright (c) 2026 Rina
