# rinova-proxy-sdk

Rust SDK for converting proxy subscription links to Clash configuration.

See the [repository README](../README.md) for full documentation.

```rust
use rinova_proxy_sdk::{convert, convert_from_lines, ConvertOptions, RuleMode};

// Online
let result = convert("https://jms-sub-url", None).await?;

// Offline
let result = convert_from_lines(&["ss://...", "trojan://..."], None)?;
```

## Modules

| Module | API |
|--------|-----|
| crate root | `convert()`, `convert_from_lines()` |
| `parser` | `parse_uri()`, `parse_lines()` |
| `fetch` | `fetch_subscription()`, `deduplicate_names()` |
| `builder` | `build_config()`, `to_yaml()` |
| `server` | `start_server()` |
| `i18n` | `t()`, `get_lang()` |
