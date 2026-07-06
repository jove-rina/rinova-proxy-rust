# Changelog

## [1.0.0] — 2026-07-06

Initial Rust release of **Rinova Proxy** by **Rina**.

### Crates

- **`rinova-proxy-sdk@1.0.0`** — Rust SDK library
- **`rinova-proxy-cli@1.0.0`** — CLI binary (`rinova-proxy-cli`)

### Features

- **Protocol parsing**: Shadowsocks (SIP002 + Legacy), VMess, Trojan, Hysteria2 (`hysteria2://` + `hy2://`)
- **SDK API**: `convert()`, `convert_from_lines()`, plus `parser`, `fetch`, `builder`, `server` modules
- **CLI**: Single-shot conversion (`-u`), HTTP subscription service (`-p`), merge mode (`--merge`), builtin/external rules
- **Policy groups**: ACL4SSR-style chained routing
- **HTTP service**: `/clash.yaml`, `/health`, `POST /refresh`, CORS enabled
- **i18n**: en/zh via `LANG`/`LC_ALL`; SDK exports `t()`, `get_lang()`, `t_with_fallback()`

### Install

```bash
cargo install rinova-proxy-cli
cargo add rinova-proxy-sdk
```
