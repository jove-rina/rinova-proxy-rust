# Rinova Proxy (Rust)

Convert subscription links to Clash configuration files.

> **v1.0.0** — `rinova-proxy-sdk` + `rinova-proxy-cli`

Node.js version: [rinova-proxy](https://github.com/jove-rina/rinova-proxy)

## Installation

```bash
# CLI
cargo install rinova-proxy-cli

# SDK
cargo add rinova-proxy-sdk
```

## CLI Usage

```bash
proxy-cli -u "https://your-jms-subscription-url"
proxy-cli -u "https://..." -o ./clash-config.yaml
proxy-cli -u "https://..." --rules external
proxy-cli -u "https://..." --merge ~/.config/clash-verge-rev/profiles/current.yaml
proxy-cli -p 25500 -u "https://..." -i 60
```

### HTTP subscription service

Verge Rev → Profiles → Import → Remote subscription:

```
URL: http://127.0.0.1:25500/clash.yaml
Update interval: 60
```

| Path | Description |
|------|-------------|
| `/clash.yaml` | Clash config (YAML) |
| `/health` | Health check JSON |
| `/refresh` | POST to trigger manual refresh |

## SDK Usage

```rust
use rinova_proxy_sdk::convert;

#[tokio::main]
async fn main() -> Result<(), rinova_proxy_sdk::ProxyError> {
    let result = convert("https://jms-sub-url", None).await?;
    std::fs::write("clash.yaml", result.yaml)?;
    Ok(())
}
```

## Supported Protocols

| Protocol | Status |
|----------|--------|
| Shadowsocks (SS) | ✅ SIP002 + Legacy |
| VMess | ✅ ws / tcp / grpc / h2 / quic / kcp |
| Trojan | ✅ |
| Hysteria2 | ✅ `hysteria2://` + `hy2://` |

## Development

```bash
cargo test
cargo run -p rinova-proxy-cli -- -u "https://..."
cargo publish -p rinova-proxy-sdk --dry-run
cargo publish -p rinova-proxy-cli --dry-run
```

## Publish order

1. `cargo publish -p rinova-proxy-sdk`
2. `cargo publish -p rinova-proxy-cli`

## License

MIT — see [LICENSE](./LICENSE)
