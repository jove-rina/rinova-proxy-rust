# Rinova Proxy (Rust)

Convert proxy subscription links to Clash configuration files.

> **v1.0.0** — `rinova-proxy-sdk` + `rinova-proxy-cli`  
> Author: **Rina**

Node.js version: [rinova-proxy](https://github.com/jove-rina/rinova-proxy)

## Crates

| Crate | Description | Documentation |
|-------|-------------|---------------|
| [`rinova-proxy-sdk`](./rinova-proxy-sdk/) | Rust library | [SDK README](./rinova-proxy-sdk/README.md) |
| [`rinova-proxy-cli`](./rinova-proxy-cli/) | CLI binary (`rinova-proxy-cli`) | [CLI README](./rinova-proxy-cli/README.md) |

## Installation

```bash
# CLI (installs binary `rinova-proxy-cli`)
cargo install rinova-proxy-cli

# SDK (add to your Cargo project)
cargo add rinova-proxy-sdk
```

Build from source:

```bash
git clone git@github.com:jove-rina/rinova-proxy-rust.git
cd rinova-proxy-rust
cargo build --release
```

## Quick Start

**CLI — one-shot conversion:**

```bash
rinova-proxy-cli -u "https://your-jms-subscription-url"
```

**SDK — programmatic conversion:**

```rust
use rinova_proxy_sdk::convert;

#[tokio::main]
async fn main() -> Result<(), rinova_proxy_sdk::ProxyError> {
    let result = convert("https://your-jms-subscription-url", None).await?;
    std::fs::write("clash.yaml", result.yaml)?;
    Ok(())
}
```

## Supported Protocols

| Protocol | URI prefix | Notes |
|----------|------------|-------|
| Shadowsocks | `ss://` | SIP002, Legacy, JMS extension |
| VMess | `vmess://` | ws / tcp / grpc / h2 / quic / kcp + tls |
| Trojan | `trojan://` | Standard |
| Hysteria2 | `hysteria2://`, `hy2://` | bandwidth `up` / `down` |

## Features

- Parse SS / VMess / Trojan / Hysteria2 subscription lines
- HTTP subscription service (`-p`) for Clash Verge Rev auto-refresh
- Node name deduplication (`-2`, `-3` suffixes)
- ACL4SSR-style chained policy groups
- Merge into existing Clash config (`--merge`)
- Subscription URL masking in logs
- i18n: en / zh via `LANG` or `LC_ALL`

## Internationalization

CLI messages and `--help` text adapt to your system language. If `LANG` or `LC_ALL` starts with `zh`, Chinese is used; otherwise English.

```bash
LANG=zh_CN.UTF-8 rinova-proxy-cli --help
LANG=en_US.UTF-8 rinova-proxy-cli -u "https://..."
```

SDK:

```rust
use rinova_proxy_sdk::{t, get_lang, Lang};

assert!(matches!(get_lang(), Lang::En | Lang::Zh));
println!("{}", t("parsed", &[("count", "5")]));
```

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

MIT — Copyright (c) 2026 Rina. See [LICENSE](./LICENSE).

中文文档：[README.zh.md](./README.zh.md)
