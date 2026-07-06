use crate::error::{ProxyError, Result};
use crate::i18n::t;
use crate::utils::decode_base64_utf8;

const USER_AGENT: &str = "@rinova/proxy-sdk/1.0.0";

pub async fn fetch_subscription(url: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let mut raw = resp.trim().to_string();

    if raw.contains('%') {
        if let Ok(decoded) = urlencoding::decode(&raw) {
            raw = decoded.into_owned();
        }
    }

    let decoded = decode_base64_utf8(&raw).unwrap_or(raw);

    let lines: Vec<String> = decoded
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();

    if lines.is_empty() {
        return Err(ProxyError::msg(t("err_empty_subscription", &[])));
    }

    Ok(lines)
}

pub fn deduplicate_names(nodes: &mut [crate::types::ProxyNode]) {
    let mut seen = std::collections::HashMap::<String, u32>::new();
    for node in nodes.iter_mut() {
        let base = strip_numeric_suffix(&node.name).to_string();
        let count = seen.entry(base.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            node.name = format!("{base}-{}", *count);
        }
    }
}

fn strip_numeric_suffix(name: &str) -> &str {
    if let Some(idx) = name.rfind('-') {
        let suffix = &name[idx + 1..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            return &name[..idx];
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProxyNode, ProxyType};

    #[test]
    fn deduplicate_preserves_unique_names() {
        let mut nodes = vec![
            ProxyNode::new("日本-01".into(), ProxyType::Ss, "a.com".into(), 443),
            ProxyNode::new("新加坡-01".into(), ProxyType::Ss, "b.com".into(), 443),
        ];
        deduplicate_names(&mut nodes);
        assert_eq!(nodes[0].name, "日本-01");
        assert_eq!(nodes[1].name, "新加坡-01");
    }

    #[test]
    fn pad_base64_roundtrip() {
        let raw = "ss://line1\nss://line2";
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(raw);
        let decoded = decode_base64_utf8(&encoded).unwrap();
        assert_eq!(decoded, raw);
    }
}
