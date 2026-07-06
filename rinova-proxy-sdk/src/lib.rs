//! Convert proxy subscription links to Clash configuration files.
//!
//! # Quick start
//!
//! ```no_run
//! use rinova_proxy_sdk::convert;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rinova_proxy_sdk::ProxyError> {
//!     let result = convert("https://your-subscription-url", None).await?;
//!     println!("parsed {} nodes", result.nodes.len());
//!     Ok(())
//! }
//! ```
//!
//! # Modules
//!
//! - [`parser`] — parse SS / VMess / Trojan / Hysteria2 URIs
//! - [`fetch`] — fetch and decode subscription content
//! - [`builder`] — assemble Clash YAML config
//! - [`server`] — HTTP subscription service
//! - [`i18n`] — localized messages (en/zh)

pub mod builder;
pub mod error;
pub mod fetch;
pub mod i18n;
pub mod parser;
pub mod server;
pub mod types;
pub mod utils;

pub use builder::{build_config, to_yaml};
pub use error::{ProxyError, Result};
pub use fetch::{deduplicate_names, fetch_subscription};
pub use i18n::{get_lang, t, t_simple, t_with_fallback, Lang};
pub use parser::{parse_lines, parse_uri};
pub use server::{start_server, ServerHandle, ServerOptions};
pub use types::{
    ClashConfig, ConvertOptions, ConvertResult, ProxyGroup, ProxyNode, ProxyType, RuleMode,
};

use builder::{build_config as build, to_yaml as serialize_yaml};
use fetch::{deduplicate_names as dedup, fetch_subscription as fetch};
use i18n::t as translate;
use parser::parse_lines as parse;

/// Fetch a subscription URL and convert it to a Clash configuration.
pub async fn convert(url: &str, opts: Option<ConvertOptions>) -> Result<ConvertResult> {
    let opts = opts.unwrap_or_default();
    let lines = fetch(url).await?;
    convert_from_parsed_lines(lines, opts, "err_no_nodes_subscription")
}

/// Convert pre-parsed URI lines without network requests.
pub fn convert_from_lines(lines: &[impl AsRef<str>], opts: Option<ConvertOptions>) -> Result<ConvertResult> {
    let lines: Vec<String> = lines.iter().map(|l| l.as_ref().to_string()).collect();
    convert_from_parsed_lines(lines, opts.unwrap_or_default(), "err_no_nodes_input")
}

fn convert_from_parsed_lines(
    lines: Vec<String>,
    opts: ConvertOptions,
    empty_err_key: &str,
) -> Result<ConvertResult> {
    let mut nodes = parse(&lines);

    if nodes.is_empty() {
        return Err(ProxyError::msg(translate(empty_err_key, &[])));
    }

    if opts.deduplicate {
        dedup(&mut nodes);
    }

    let config = build(&nodes, opts.rules);
    let yaml = serialize_yaml(&config)?;

    Ok(ConvertResult {
        config,
        yaml,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_from_lines_returns_yaml_config_nodes() {
        let lines = vec![
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#US-01".to_string(),
            "trojan://pass@sg.example.com:443?security=tls&sni=sg.example.com#SG-01".to_string(),
        ];
        let result = convert_from_lines(&lines, None).unwrap();
        assert_eq!(result.nodes.len(), 2);
        assert!(result.yaml.contains("proxies:"));
        assert!(result.yaml.contains("US-01"));
    }

    #[test]
    fn convert_from_lines_deduplicates_by_default() {
        let lines = vec![
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@a.com:443#Node".to_string(),
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@b.com:443#Node".to_string(),
        ];
        let result = convert_from_lines(&lines, None).unwrap();
        assert_eq!(result.nodes[0].name, "Node");
        assert_eq!(result.nodes[1].name, "Node-2");
    }

    #[test]
    fn convert_from_lines_supports_external_rules() {
        let lines = vec![
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#US-01".to_string(),
        ];
        let result = convert_from_lines(
            &lines,
            Some(ConvertOptions {
                rules: RuleMode::External,
                deduplicate: true,
            }),
        )
        .unwrap();
        assert!(result.config.rules[0].starts_with("RULE-SET"));
    }

    #[test]
    fn convert_from_lines_throws_on_empty() {
        assert!(convert_from_lines(&[] as &[String], None).is_err());
    }

    #[test]
    fn convert_from_lines_yaml_contains_groups_and_rules() {
        let lines = vec![
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#US-01".to_string(),
            "trojan://pass@sg.example.com:443?security=tls&sni=sg.example.com#SG-01".to_string(),
        ];
        let result = convert_from_lines(&lines, None).unwrap();
        assert!(result.yaml.contains("proxy-groups:"));
        assert!(result.yaml.contains("rules:"));
        assert!(result.yaml.contains("SG-01"));
    }

    #[test]
    fn convert_from_lines_skips_dedup_when_false() {
        let lines = vec![
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@a.com:443#Node".to_string(),
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@b.com:443#Node".to_string(),
        ];
        let result = convert_from_lines(
            &lines,
            Some(ConvertOptions {
                rules: RuleMode::Builtin,
                deduplicate: false,
            }),
        )
        .unwrap();
        assert_eq!(result.nodes[0].name, "Node");
        assert_eq!(result.nodes[1].name, "Node");
    }

    #[test]
    fn convert_from_lines_defaults_to_builtin_rules() {
        let lines = vec![
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#US-01".to_string(),
        ];
        let result = convert_from_lines(&lines, None).unwrap();
        assert!(result.config.rules[0].starts_with("DOMAIN-SUFFIX"));
    }

    #[test]
    fn convert_from_lines_throws_when_all_unparseable() {
        assert!(convert_from_lines(&["invalid://line".to_string()], None).is_err());
    }

    #[test]
    fn parse_uri_and_build_config_to_yaml() {
        let node = parse_uri("ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@host:8388#Test").unwrap();
        let config = build_config(&[node], RuleMode::Builtin);
        let yaml = to_yaml(&config).unwrap();
        assert!(yaml.contains("Test"));
    }
}
