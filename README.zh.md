# Rinova Proxy (Rust)

订阅链接 → Clash 配置文件转换工具。

> **v1.0.0** — `rinova-proxy-sdk` + `rinova-proxy-cli`  
> 作者：**Rina**

Node.js 版本：[rinova-proxy](https://github.com/jove-rina/rinova-proxy)

English: [README.md](./README.md)

## Crates

| Crate | 说明 | 文档 |
|-------|------|------|
| [`rinova-proxy-sdk`](./rinova-proxy-sdk/) | Rust 库 | [SDK README](./rinova-proxy-sdk/README.md) |
| [`rinova-proxy-cli`](./rinova-proxy-cli/) | CLI 二进制（`rinova-proxy-cli`） | [CLI README](./rinova-proxy-cli/README.md) |

## 安装

```bash
# CLI（安装二进制 `rinova-proxy-cli`）
cargo install rinova-proxy-cli

# SDK（添加到 Cargo 项目）
cargo add rinova-proxy-sdk
```

从源码构建：

```bash
git clone git@github.com:jove-rina/rinova-proxy-rust.git
cd rinova-proxy-rust
cargo build --release
```

## 快速开始

**CLI — 一次性转换：**

```bash
rinova-proxy-cli -u "https://your-jms-subscription-url"
```

**SDK — 程序化转换：**

```rust
use rinova_proxy_sdk::convert;

#[tokio::main]
async fn main() -> Result<(), rinova_proxy_sdk::ProxyError> {
    let result = convert("https://your-jms-subscription-url", None).await?;
    std::fs::write("clash.yaml", result.yaml)?;
    Ok(())
}
```

## 支持协议

| 协议 | URI 前缀 | 说明 |
|------|----------|------|
| Shadowsocks | `ss://` | SIP002、Legacy、JMS 扩展 |
| VMess | `vmess://` | ws / tcp / grpc / h2 / quic / kcp + tls |
| Trojan | `trojan://` | 标准格式 |
| Hysteria2 | `hysteria2://`、`hy2://` | 带宽参数 `up` / `down` |

## 功能

- 解析 SS / VMess / Trojan / Hysteria2 订阅行
- HTTP 订阅服务（`-p`），供 Clash Verge Rev 自动刷新
- 节点名去重（同名追加 `-2`、`-3` 后缀）
- ACL4SSR 风格链式策略组
- 合并到现有 Clash 配置（`--merge`）
- 日志中订阅 URL 脱敏
- 国际化：通过 `LANG` 或 `LC_ALL` 切换 en / zh

## 策略组与分流

生成的 Clash 配置采用 ACL4SSR 风格的**链式策略组**：

```
规则 MATCH / 国外域名
    ↓
🌍 国外网站（默认第一项）
    ↓
🚀 节点选择 ← 在 Verge 中切换节点
    ↓
具体 JMS 节点
```

| 策略组 | 作用 |
|--------|------|
| `🚀 节点选择` | 主选节点，Verge 默认展示此组 |
| `♻️ 自动选择` | url-test 测速，选延迟最低节点 |
| `🎯 直连` | 直连或经节点代理 |
| `🌍 国外网站` | 规则引用的国外分流组，**默认跟随 `🚀 节点选择`** |
| `🇨🇳 国内网站` | 国内域名分流 |
| `🛑 广告拦截` | 广告域名 REJECT |

## 国际化

CLI 消息与 `--help` 文本根据系统语言自动切换。`LANG` 或 `LC_ALL` 以 `zh` 开头时使用中文，否则使用英文。

```bash
LANG=zh_CN.UTF-8 rinova-proxy-cli --help
LANG=en_US.UTF-8 rinova-proxy-cli -u "https://..."
```

SDK：

```rust
use rinova_proxy_sdk::{t, get_lang, Lang};

assert!(matches!(get_lang(), Lang::En | Lang::Zh));
println!("{}", t("parsed", &[("count", "5")]));
```

## 与 Node 版对照

| 项目 | Node | Rust |
|------|------|------|
| SDK 包名 | `@rinova/proxy-sdk` | `rinova-proxy-sdk` |
| CLI 命令 | `proxy-cli` | `rinova-proxy-cli` |
| 版本 | 2.0.0 | 1.0.0（Rust 首版） |
| 安装 | `npm install -g @rinova/proxy-cli` | `cargo install rinova-proxy-cli` |

## 开发

```bash
cargo test
cargo run -p rinova-proxy-cli -- -u "https://..."
cargo publish -p rinova-proxy-sdk --dry-run
cargo publish -p rinova-proxy-cli --dry-run
```

### 测试覆盖

| 套件 | 位置 | 说明 |
|------|------|------|
| parser | `rinova-proxy-sdk/src/parser.rs` | SS / VMess / Trojan / Hy2 解析 |
| builder | `rinova-proxy-sdk/src/builder.rs` | 策略组与规则 |
| sdk | `rinova-proxy-sdk/src/lib.rs` | `convert_from_lines` 等 |
| server | `rinova-proxy-sdk/src/server.rs` | HTTP 端点 |
| fetch / utils | `fetch.rs` / `utils.rs` | 去重、Base64 |

## 发布顺序

1. `cargo publish -p rinova-proxy-sdk`
2. `cargo publish -p rinova-proxy-cli`

## 变更记录

见 [CHANGELOG.md](./CHANGELOG.md)。

## License

MIT — Copyright (c) 2026 Rina。详见 [LICENSE](./LICENSE)。
