# rinova-proxy-sdk

Proxy subscription to Clash config converter — Rust SDK.

> **v1.0.0** · Author: **Rina**

## Install

```toml
[dependencies]
rinova-proxy-sdk = "1.0.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```bash
cargo add rinova-proxy-sdk
```

## Quick Start

```rust
use rinova_proxy_sdk::convert;

#[tokio::main]
async fn main() -> Result<(), rinova_proxy_sdk::ProxyError> {
    let result = convert("https://jms-subscription-url", None).await?;
    println!("parsed {} nodes", result.nodes.len());
    std::fs::write("clash.yaml", result.yaml)?;
    Ok(())
}
```

---

## API Reference

### `convert(url, opts) -> Result<ConvertResult>`

Fetch a subscription URL, parse proxy nodes, and generate a complete Clash configuration.

```rust
use rinova_proxy_sdk::{convert, ConvertOptions, RuleMode};

// Basic
let result = convert("https://jms-api.example.com/sub?token=xxx", None).await?;

// With options
let result = convert(
    "https://...",
    Some(ConvertOptions {
        rules: RuleMode::External,  // default: RuleMode::Builtin
        deduplicate: false,         // default: true
    }),
).await?;

let rinova_proxy_sdk::ConvertResult { yaml, config, nodes } = result;
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | `&str` | required | Proxy subscription URL |
| `opts.rules` | `RuleMode` | `Builtin` | `Builtin` or `External` (ACL4SSR RuleSet) |
| `opts.deduplicate` | `bool` | `true` | Append `-2`, `-3` to duplicate node names |

**Returns** `ConvertResult`:

| Field | Type | Description |
|-------|------|-------------|
| `yaml` | `String` | Clash config as YAML |
| `config` | `ClashConfig` | Full config object |
| `nodes` | `Vec<ProxyNode>` | Parsed proxy nodes |

**Errors** `ProxyError` if subscription is empty or no valid nodes are found.

---

### `convert_from_lines(lines, opts) -> Result<ConvertResult>`

Convert URI lines **without network requests**.

```rust
use rinova_proxy_sdk::{convert_from_lines, ConvertOptions, RuleMode};

let lines = [
    "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#US-01",
    "trojan://pass@sg.example.com:443?sni=sg.example.com#SG-01",
];

let result = convert_from_lines(
    &lines,
    Some(ConvertOptions {
        rules: RuleMode::External,
        deduplicate: true,
    }),
)?;
```

Same options and return type as `convert()`.

---

### `parse_uri(uri) -> Result<ProxyNode>`

Parse a single proxy URI.

```rust
use rinova_proxy_sdk::parse_uri;

let node = parse_uri("ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@host:8388#MyNode")?;
assert_eq!(node.name, "MyNode");
assert_eq!(node.proxy_type.as_str(), "ss");
assert_eq!(node.port, 8388);

// Protocol-specific fields live in `extra`
let cipher = node.extra.get("cipher").and_then(|v| v.as_str());
let password = node.extra.get("password").and_then(|v| v.as_str());
```

**Supported prefixes:** `ss://`, `vmess://`, `trojan://`, `hysteria2://`, `hy2://`

---

### `parse_lines(lines) -> Vec<ProxyNode>`

Parse multiple URI lines. Invalid lines are skipped with a warning to stderr.

```rust
use rinova_proxy_sdk::parse_lines;

let nodes = parse_lines(&[
    "ss://YWVz...@host1:8388#Node1",
    "invalid-uri",              // skipped
    "trojan://pass@host2:443#Node2",
]);
assert_eq!(nodes.len(), 2);
```

---

### `fetch_subscription(url) -> Result<Vec<String>>`

Fetch and decode a subscription into URI lines (Base64 / URL-decode tolerant).

```rust
use rinova_proxy_sdk::fetch_subscription;

#[tokio::main]
async fn main() -> Result<(), rinova_proxy_sdk::ProxyError> {
    let lines = fetch_subscription("https://jms-api.example.com/sub").await?;
    for line in &lines {
        println!("{line}");
    }
    Ok(())
}
```

User-Agent: `@rinova/proxy-sdk/1.0.0`

---

### `deduplicate_names(nodes: &mut [ProxyNode])`

Rename duplicate nodes in-place: first keeps original name, others get `-2`, `-3`, …

```rust
use rinova_proxy_sdk::{deduplicate_names, ProxyNode, ProxyType};

let mut nodes = vec![
    ProxyNode::new("Japan".into(), ProxyType::Ss, "a.com".into(), 443),
    ProxyNode::new("Japan".into(), ProxyType::Ss, "b.com".into(), 443),
];
deduplicate_names(&mut nodes);
// → "Japan", "Japan-2"
```

---

### `build_config(nodes, rule_mode) -> ClashConfig`

Build a Clash config from parsed nodes.

```rust
use rinova_proxy_sdk::{build_config, parse_lines, RuleMode};

let nodes = parse_lines(&["ss://..."]);
let config = build_config(&nodes, RuleMode::Builtin);
// config.proxies, config.proxy_groups, config.rules
```

**Policy groups** (ACL4SSR-style chained routing):

```
Rule MATCH / foreign domains
    ↓
🌍 国外网站  (default: 🚀 节点选择)
    ↓
🚀 节点选择  ← switch nodes in Verge
    ↓
specific proxy node
```

| Group | Purpose |
|-------|---------|
| `🚀 节点选择` | Main node selector |
| `♻️ 自动选择` | url-test, lowest latency |
| `🎯 直连` | Direct or proxy |
| `🌍 国外网站` | Referenced by rules, follows `🚀 节点选择` |
| `🇨🇳 国内网站` | Domestic routing |
| `🛑 广告拦截` | REJECT |

---

### `to_yaml(config) -> Result<String>`

Serialize `ClashConfig` to YAML.

```rust
use rinova_proxy_sdk::{build_config, to_yaml, RuleMode};

let yaml = to_yaml(&build_config(&nodes, RuleMode::Builtin))?;
std::fs::write("clash.yaml", yaml)?;
```

---

### `start_server(opts) -> Result<ServerHandle>`

Start an HTTP server with automatic subscription refresh.

```rust
use rinova_proxy_sdk::{start_server, ServerOptions, RuleMode};

#[tokio::main]
async fn main() -> Result<(), rinova_proxy_sdk::ProxyError> {
    let handle = start_server(ServerOptions {
        url: "https://jms-api.example.com/sub".into(),
        port: 25500,
        interval_min: 60,
        rule_mode: RuleMode::Builtin,
    }).await?;

    // Block until Ctrl+C / SIGTERM
    tokio::signal::ctrl_c().await.ok();
    handle.shutdown().await;
    Ok(())
}
```

**HTTP endpoints** (listen on `127.0.0.1:{port}`):

| Method | Path | Description |
|--------|------|-------------|
| GET | `/clash.yaml` | Clash config YAML |
| GET | `/health` | JSON: `status`, `nodes`, `updatedAt`, `nextRefreshMin`, `lastError` |
| POST | `/refresh` | Manual refresh → `{ ok, skipped, nodes }` |
| GET | `/refresh` | 405 Method Not Allowed |

All responses include `Access-Control-Allow-Origin: *`.

---

### `t(key, params)` / `t_with_fallback(key, params, fallback)` / `get_lang()`

Localized messages based on `LANG` / `LC_ALL`.

```rust
use rinova_proxy_sdk::{t, t_with_fallback, get_lang, Lang};

match get_lang() {
    Lang::Zh => println!("中文环境"),
    Lang::En => println!("English"),
}

println!("{}", t("refreshing", &[]));
println!("{}", t("parsed", &[("count", "5")]));
println!("{}", t_with_fallback("missing_key", &[], Some("fallback text")));
```

---

## Types

```rust
use rinova_proxy_sdk::{
    ClashConfig, ConvertOptions, ConvertResult,
    ProxyGroup, ProxyNode, ProxyType, RuleMode,
    ProxyError, Result,
};
```

| Type | Description |
|------|-------------|
| `ProxyNode` | `name`, `proxy_type`, `server`, `port`, `extra` (protocol fields) |
| `ProxyGroup` | Policy group (`name`, `group_type`, `proxies`, …) |
| `ClashConfig` | Full config (`proxies`, `proxy_groups`, `rules`, ports) |
| `ConvertOptions` | `{ rules: RuleMode, deduplicate: bool }` |
| `ConvertResult` | `{ config, yaml, nodes }` |
| `RuleMode` | `Builtin` \| `External` |
| `ProxyType` | `Ss` \| `Vmess` \| `Trojan` \| `Hysteria2` |
| `ServerOptions` | `{ url, port, interval_min, rule_mode }` |
| `ServerHandle` | Call `.shutdown().await` to stop |
| `Lang` | `En` \| `Zh` |
| `ProxyError` | Error enum |
| `Result<T>` | `std::result::Result<T, ProxyError>` |

### `ProxyNode.extra` fields (by protocol)

Protocol-specific options are stored in `extra` (`IndexMap<String, Value>`):

| Protocol | Common keys in `extra` |
|----------|------------------------|
| SS | `cipher`, `password` |
| VMess | `uuid`, `alterId`, `cipher`, `network`, `tls`, `sni`, `ws-opts`, `grpc-opts`, … |
| Trojan | `password`, `sni`, `skip-cert-verify` |
| Hysteria2 | `password`, `sni`, `alpn`, `up`, `down`, `skip-cert-verify` |

---

## Modules

| Module | Exports |
|--------|---------|
| crate root | `convert`, `convert_from_lines`, re-exports |
| `parser` | `parse_uri`, `parse_lines` |
| `fetch` | `fetch_subscription`, `deduplicate_names` |
| `builder` | `build_config`, `to_yaml` |
| `server` | `start_server`, `ServerHandle`, `ServerOptions` |
| `i18n` | `t`, `t_with_fallback`, `get_lang`, `Lang` |
| `types` | All public types |
| `error` | `ProxyError`, `Result` |

---

## License

MIT — Copyright (c) 2026 Rina
