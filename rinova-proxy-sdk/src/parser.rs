use crate::error::{ProxyError, Result};
use crate::i18n::t;
use crate::types::{ProxyNode, ProxyType};
use crate::utils::{decode_base64_utf8, safe_decode_uri};
use indexmap::IndexMap;
use serde_yaml::Value as YamlValue;

fn parse_ss(uri: &str) -> Result<ProxyNode> {
    let without_scheme = &uri[5..];

    let hash_idx = without_scheme.rfind('#');
    let raw_name = match hash_idx {
        Some(idx) => safe_decode_uri(Some(&without_scheme[idx + 1..])),
        None => String::new(),
    };
    let body = match hash_idx {
        Some(idx) => &without_scheme[..idx],
        None => without_scheme,
    };

    if let Some(at_idx) = body.find('@') {
        let b64_part = &body[..at_idx];
        let host_port = &body[at_idx + 1..];
        let (server, port) = split_host_port(host_port).ok_or_else(|| {
            ProxyError::msg(t("err_ss_address", &[("host", host_port)]))
        })?;

        let decoded = decode_base64_utf8(b64_part).map_err(|_| {
            ProxyError::msg(t("err_ss_sip002_credentials", &[("raw", b64_part)]))
        })?;

        if let Some(at_in_decoded) = decoded.find('@') {
            let cred = &decoded[..at_in_decoded];
            let (method, password) = split_cred(cred).ok_or_else(|| {
                ProxyError::msg(t(
                    "err_ss_sip002_credentials",
                    &[("raw", cred)],
                ))
            })?;
            let mut node = ProxyNode::new(
                if raw_name.is_empty() {
                    server.clone()
                } else {
                    raw_name
                },
                ProxyType::Ss,
                server,
                port,
            );
            node.set_str("cipher", method);
            node.set_str("password", password);
            return Ok(node);
        }

        let colon_idx = decoded.find(':').ok_or_else(|| {
            ProxyError::msg(t(
                "err_ss_sip002_credentials",
                &[("raw", &decoded)],
            ))
        })?;
        let mut node = ProxyNode::new(
            if raw_name.is_empty() {
                server.clone()
            } else {
                raw_name
            },
            ProxyType::Ss,
            server,
            port,
        );
        node.set_str("cipher", &decoded[..colon_idx]);
        node.set_str("password", &decoded[colon_idx + 1..]);
        return Ok(node);
    }

    let decoded = decode_base64_utf8(body).map_err(|_| {
        ProxyError::msg(t("err_ss_legacy_no_at", &[("raw", body)]))
    })?;
    let at_in_decoded = decoded.find('@').ok_or_else(|| {
        ProxyError::msg(t("err_ss_legacy_no_at", &[("raw", &decoded)]))
    })?;
    let cred = &decoded[..at_in_decoded];
    let host_port = &decoded[at_in_decoded + 1..];
    let (server, port) = split_host_port(host_port).ok_or_else(|| {
        ProxyError::msg(t("err_ss_legacy_address", &[("host", host_port)]))
    })?;
    let (method, password) = split_cred(cred).ok_or_else(|| {
        ProxyError::msg(t("err_ss_legacy_no_method", &[("raw", cred)]))
    })?;

    let mut node = ProxyNode::new(
        if raw_name.is_empty() {
            server.clone()
        } else {
            raw_name
        },
        ProxyType::Ss,
        server,
        port,
    );
    node.set_str("cipher", method);
    node.set_str("password", password);
    Ok(node)
}

fn indexmap_to_yaml_mapping(map: IndexMap<String, YamlValue>) -> YamlValue {
    let mut mapping = serde_yaml::Mapping::new();
    for (k, v) in map {
        mapping.insert(YamlValue::String(k), v);
    }
    YamlValue::Mapping(mapping)
}

fn split_host_port(host_port: &str) -> Option<(String, u16)> {
    let (server, port_str) = host_port.rsplit_once(':')?;
    let port = port_str.parse().ok()?;
    if server.is_empty() {
        return None;
    }
    Some((server.to_string(), port))
}

fn split_cred(cred: &str) -> Option<(String, String)> {
    let colon_idx = cred.find(':')?;
    let method = cred[..colon_idx].trim();
    if method.is_empty() {
        return None;
    }
    Some((method.to_string(), cred[colon_idx + 1..].to_string()))
}

fn vmess_field_str(obj: &serde_json::Value, key: &str) -> Option<String> {
    match obj.get(key)? {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn vmess_field_u16(obj: &serde_json::Value, key: &str) -> Option<u16> {
    match obj.get(key)? {
        serde_json::Value::Number(n) => n.as_u64().and_then(|v| u16::try_from(v).ok()),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn parse_vmess(uri: &str) -> Result<ProxyNode> {
    let b64 = &uri[8..];
    let json_str = decode_base64_utf8(b64).map_err(|_| ProxyError::msg(t("err_vmess_parse", &[])))?;
    let obj: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|_| ProxyError::msg(t("err_vmess_parse", &[])))?;

    let ps = vmess_field_str(&obj, "ps");
    let remarks = vmess_field_str(&obj, "remarks");
    let name = {
        let raw = safe_decode_uri(ps.as_deref().or(remarks.as_deref()));
        if raw.is_empty() {
            "Unnamed".to_string()
        } else {
            raw
        }
    };
    let ws_host = vmess_field_str(&obj, "host");
    let server = vmess_field_str(&obj, "add")
        .or_else(|| vmess_field_str(&obj, "host"))
        .unwrap_or_default();
    let port = vmess_field_u16(&obj, "port").unwrap_or(0);

    if server.is_empty() || port == 0 {
        return Err(ProxyError::msg(t("err_vmess_no_server", &[])));
    }

    let mut node = ProxyNode::new(name, ProxyType::Vmess, server, port);
    if let Some(uuid) = vmess_field_str(&obj, "id") {
        node.set_str("uuid", uuid);
    }
    node.set_u64(
        "alterId",
        vmess_field_u16(&obj, "aid").map(u64::from).unwrap_or(0),
    );
    node.set_str(
        "cipher",
        vmess_field_str(&obj, "scy").as_deref().unwrap_or("auto"),
    );

    let net = vmess_field_str(&obj, "net").unwrap_or_else(|| "tcp".to_string());
    match net.as_str() {
        "ws" => {
            node.set_str("network", "ws");
            let mut ws_opts = IndexMap::new();
            ws_opts.insert(
                "path".to_string(),
                YamlValue::String(vmess_field_str(&obj, "path").unwrap_or_else(|| "/".to_string())),
            );
            if let Some(host) = ws_host.clone() {
                let mut headers = IndexMap::new();
                headers.insert("Host".to_string(), YamlValue::String(host));
                ws_opts.insert("headers".to_string(), indexmap_to_yaml_mapping(headers));
            }
            node.set_value("ws-opts", indexmap_to_yaml_mapping(ws_opts));
        }
        "grpc" => {
            node.set_str("network", "grpc");
            let mut grpc_opts = IndexMap::new();
            grpc_opts.insert(
                "grpc-service-name".to_string(),
                YamlValue::String(vmess_field_str(&obj, "serviceName").unwrap_or_default()),
            );
            node.set_value("grpc-opts", indexmap_to_yaml_mapping(grpc_opts));
        }
        "h2" => {
            node.set_str("network", "h2");
            let mut h2_opts = IndexMap::new();
            h2_opts.insert(
                "path".to_string(),
                YamlValue::String(vmess_field_str(&obj, "path").unwrap_or_else(|| "/".to_string())),
            );
            if let Some(host) = ws_host.clone() {
                h2_opts.insert("host".to_string(), YamlValue::String(host));
            }
            node.set_value("h2-opts", indexmap_to_yaml_mapping(h2_opts));
        }
        "quic" => {
            node.set_str("network", "quic");
            let mut quic_opts = IndexMap::new();
            quic_opts.insert(
                "security".to_string(),
                YamlValue::String(
                    vmess_field_str(&obj, "security").unwrap_or_else(|| "none".to_string()),
                ),
            );
            quic_opts.insert(
                "key".to_string(),
                YamlValue::String(vmess_field_str(&obj, "key").unwrap_or_default()),
            );
            node.set_value("quic-opts", indexmap_to_yaml_mapping(quic_opts));
        }
        "kcp" => {
            node.set_str("network", "kcp");
            let mut kcp_opts = IndexMap::new();
            kcp_opts.insert("mtu".to_string(), YamlValue::Number(1350.into()));
            kcp_opts.insert("mptcp".to_string(), YamlValue::Bool(false));
            node.set_value("kcp-opts", indexmap_to_yaml_mapping(kcp_opts));
        }
        _ => {}
    }

    let tls = vmess_field_str(&obj, "tls");
    match tls.as_deref() {
        Some("tls") => {
            node.set_bool("tls", true);
            if let Some(sni) = vmess_field_str(&obj, "sni") {
                node.set_str("sni", sni);
            }
            if let Some(alpn) = vmess_field_str(&obj, "alpn") {
                let values: Vec<YamlValue> = alpn
                    .split(',')
                    .map(|s| YamlValue::String(s.trim().to_string()))
                    .collect();
                node.set_value("alpn", YamlValue::Sequence(values));
            }
            if let Some(fp) = vmess_field_str(&obj, "fp") {
                node.set_str("client-fingerprint", fp);
            }
            let skip = ["skip-cert-verify", "skip_cert_verify", "allow_insecure", "allowInsecure"]
                .iter()
                .filter_map(|key| vmess_field_str(&obj, key))
                .any(|v| v == "true" || v == "1");
            if skip {
                node.set_bool("skip-cert-verify", true);
            }
        }
        Some("none") => {
            node.set_bool("tls", false);
            node.set_bool("skip-cert-verify", true);
        }
        _ => {}
    }

    Ok(node)
}

fn parse_trojan(uri: &str) -> Result<ProxyNode> {
    let parsed = url::Url::parse(uri)?;
    let name = {
        let raw = safe_decode_uri(parsed.fragment());
        if raw.is_empty() {
            parsed.host_str().unwrap_or_default().to_string()
        } else {
            raw
        }
    };
    let server = parsed.host_str().unwrap_or_default().to_string();
    let port = parsed.port().unwrap_or(443);
    let password = if parsed.username().is_empty() {
        parsed.password().unwrap_or_default().to_string()
    } else {
        parsed.username().to_string()
    };

    let mut node = ProxyNode::new(name, ProxyType::Trojan, server.clone(), port);
    node.set_str("password", password);
    node.set_str(
        "sni",
        parsed
            .query_pairs()
            .find(|(k, _)| k == "sni")
            .map(|(_, v)| v.into_owned())
            .unwrap_or(server),
    );

    let allow_insecure = parsed
        .query_pairs()
        .find(|(k, _)| k == "allowInsecure" || k == "allow_insecure")
        .map(|(_, v)| v.into_owned());
    if allow_insecure.as_deref() == Some("1") || allow_insecure.as_deref() == Some("true") {
        node.set_bool("skip-cert-verify", true);
    }

    Ok(node)
}

fn parse_hysteria2(uri: &str) -> Result<ProxyNode> {
    let parsed = url::Url::parse(uri)?;
    let name = {
        let raw = safe_decode_uri(parsed.fragment());
        if raw.is_empty() {
            parsed.host_str().unwrap_or_default().to_string()
        } else {
            raw
        }
    };
    let server = parsed.host_str().unwrap_or_default().to_string();
    let port = parsed.port().unwrap_or(443);
    let password = if parsed.username().is_empty() {
        parsed.password().unwrap_or_default().to_string()
    } else {
        parsed.username().to_string()
    };

    let mut node = ProxyNode::new(name, ProxyType::Hysteria2, server, port);
    node.set_str("password", password);

    if let Some(sni) = parsed
        .query_pairs()
        .find(|(k, _)| k == "sni" || k == "peer")
        .map(|(_, v)| v.into_owned())
    {
        node.set_str("sni", sni);
    }

    let insecure = parsed
        .query_pairs()
        .find(|(k, _)| k == "insecure" || k == "allowInsecure" || k == "skip-cert-verify")
        .map(|(_, v)| v.into_owned());
    if insecure.as_deref() == Some("1") || insecure.as_deref() == Some("true") {
        node.set_bool("skip-cert-verify", true);
    }

    if let Some(alpn) = parsed
        .query_pairs()
        .find(|(k, _)| k == "alpn")
        .map(|(_, v)| v.into_owned())
    {
        let values: Vec<YamlValue> = alpn
            .split(',')
            .map(|s| YamlValue::String(s.trim().to_string()))
            .collect();
        node.set_value("alpn", YamlValue::Sequence(values));
    }

    if let Some(up) = parsed
        .query_pairs()
        .find(|(k, _)| k == "up" || k == "upload")
        .and_then(|(_, v)| v.parse::<u64>().ok())
    {
        node.set_u64("up", up);
    }

    if let Some(down) = parsed
        .query_pairs()
        .find(|(k, _)| k == "down" || k == "download")
        .and_then(|(_, v)| v.parse::<u64>().ok())
    {
        node.set_u64("down", down);
    }

    Ok(node)
}

pub fn parse_uri(uri: &str) -> Result<ProxyNode> {
    if let Some(rest) = uri.strip_prefix("ss://") {
        return parse_ss(&format!("ss://{rest}"));
    }
    if let Some(rest) = uri.strip_prefix("vmess://") {
        return parse_vmess(&format!("vmess://{rest}"));
    }
    if uri.starts_with("trojan://") {
        return parse_trojan(uri);
    }
    if uri.starts_with("hysteria2://") || uri.starts_with("hy2://") {
        return parse_hysteria2(uri);
    }

    let prefix = uri.chars().take(10).collect::<String>();
    Err(ProxyError::msg(t(
        "err_unsupported_protocol",
        &[("prefix", &prefix)],
    )))
}

pub fn parse_lines(lines: &[impl AsRef<str>]) -> Vec<ProxyNode> {
    let mut nodes = Vec::new();
    for line in lines {
        match parse_uri(line.as_ref()) {
            Ok(node) => nodes.push(node),
            Err(err) => {
                eprintln!(
                    "⚠️  {}",
                    t("skip_node", &[("msg", &err.to_string())])
                );
            }
        }
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::deduplicate_names;

    #[test]
    fn ss_sip002_standard() {
        let uri = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#美国-01";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node.proxy_type, ProxyType::Ss);
        assert_eq!(node.name, "美国-01");
        assert_eq!(node.server, "us1.example.com");
        assert_eq!(node.port, 8388);
        assert_eq!(node.extra.get("cipher").and_then(|v| v.as_str()), Some("aes-256-gcm"));
        assert_eq!(node.extra.get("password").and_then(|v| v.as_str()), Some("password"));
    }

    #[test]
    fn ss_legacy_format() {
        use base64::Engine;
        let raw = "chacha20-ietf-poly1305:secret123@jp1.example.com:443";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw);
        let uri = format!("ss://{b64}#日本-01");
        let node = parse_uri(&uri).unwrap();
        assert_eq!(node.name, "日本-01");
        assert_eq!(node.server, "jp1.example.com");
        assert_eq!(node.port, 443);
    }

    #[test]
    fn ss_fallback_name_to_server() {
        let uri = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@sg1.example.com:443";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node.name, "sg1.example.com");
    }

    #[test]
    fn ss_password_with_colons() {
        use base64::Engine;
        let raw = "2022-blake3-aes-256-gcm:long:password:with:colons";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw);
        let uri = format!("ss://{b64}@hk1.example.com:443#香港");
        let node = parse_uri(&uri).unwrap();
        assert_eq!(
            node.extra.get("cipher").and_then(|v| v.as_str()),
            Some("2022-blake3-aes-256-gcm")
        );
        assert_eq!(
            node.extra.get("password").and_then(|v| v.as_str()),
            Some("long:password:with:colons")
        );
    }

    #[test]
    fn vmess_aid_as_number() {
        use base64::Engine;
        let cfg = r#"{"ps":"JMS-test","port":"6191","id":"be599ec8-c3de-47d5-ac77-c57f24e13d47","aid":0,"net":"tcp","type":"none","tls":"none","add":"104.243.21.29"}"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(cfg);
        let node = parse_uri(&format!("vmess://{b64}")).unwrap();
        assert_eq!(node.proxy_type, ProxyType::Vmess);
        assert_eq!(node.server, "104.243.21.29");
        assert_eq!(node.extra.get("alterId").and_then(|v| v.as_u64()), Some(0));
    }

    #[test]
    fn vmess_ws_tls() {
        use base64::Engine;
        let cfg = serde_json::json!({
            "v": "2",
            "ps": "日本-01",
            "add": "jp1.example.com",
            "port": "443",
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "aid": "0",
            "net": "ws",
            "host": "cloudflare.com",
            "path": "/ws?ed=2048",
            "tls": "tls",
            "sni": "cloudflare.com"
        });
        let b64 = base64::engine::general_purpose::STANDARD.encode(cfg.to_string());
        let node = parse_uri(&format!("vmess://{b64}")).unwrap();
        assert_eq!(node.proxy_type, ProxyType::Vmess);
        assert_eq!(node.extra.get("network").and_then(|v| v.as_str()), Some("ws"));
        assert_eq!(node.extra.get("tls").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn trojan_standard() {
        let uri = "trojan://my-password@sg1.example.com:443?security=tls&sni=sg1.example.com#新加坡-01";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node.proxy_type, ProxyType::Trojan);
        assert_eq!(node.name, "新加坡-01");
        assert_eq!(node.extra.get("password").and_then(|v| v.as_str()), Some("my-password"));
    }

    #[test]
    fn hysteria2_standard() {
        let uri = "hysteria2://my-password@jp1.example.com:443?insecure=1&sni=jp1.example.com&alpn=h3#日本-Hy2";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node.proxy_type, ProxyType::Hysteria2);
        assert_eq!(node.name, "日本-Hy2");
        assert_eq!(
            node.extra.get("skip-cert-verify").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn deduplicate_names_suffix() {
        let mut nodes = vec![
            ProxyNode::new("日本-01".into(), ProxyType::Ss, "a.com".into(), 443),
            ProxyNode::new("日本-01".into(), ProxyType::Ss, "b.com".into(), 443),
            ProxyNode::new("日本-01".into(), ProxyType::Ss, "c.com".into(), 443),
        ];
        deduplicate_names(&mut nodes);
        assert_eq!(nodes[0].name, "日本-01");
        assert_eq!(nodes[1].name, "日本-2");
        assert_eq!(nodes[2].name, "日本-3");
    }

    #[test]
    fn parse_lines_mixed_protocols() {
        let lines = vec![
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#美国-01".to_string(),
            "trojan://pass@sg1.example.com:443?security=tls&sni=sg1.example.com#新加坡-01".to_string(),
        ];
        let nodes = parse_lines(&lines);
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn ss_sip002_nonstandard_extension() {
        use base64::Engine;
        let full = "aes-256-gcm:RealPassword@extra.example.com:8443";
        let b64 = base64::engine::general_purpose::STANDARD.encode(full);
        let uri = format!("ss://{b64}@extra.example.com:8443#测试");
        let node = parse_uri(&uri).unwrap();
        assert_eq!(
            node.extra.get("cipher").and_then(|v| v.as_str()),
            Some("aes-256-gcm")
        );
        assert_eq!(
            node.extra.get("password").and_then(|v| v.as_str()),
            Some("RealPassword")
        );
        assert_eq!(node.server, "extra.example.com");
        assert_eq!(node.port, 8443);
    }

    #[test]
    fn vmess_grpc() {
        use base64::Engine;
        let cfg = serde_json::json!({
            "v": "2",
            "ps": "日本-01",
            "add": "jp1.example.com",
            "port": "443",
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "aid": "0",
            "net": "grpc",
            "serviceName": "my-service",
            "tls": "tls"
        });
        let b64 = base64::engine::general_purpose::STANDARD.encode(cfg.to_string());
        let node = parse_uri(&format!("vmess://{b64}")).unwrap();
        assert_eq!(node.extra.get("network").and_then(|v| v.as_str()), Some("grpc"));
        let grpc_opts = node.extra.get("grpc-opts").unwrap();
        let mapping = grpc_opts.as_mapping().unwrap();
        let key = YamlValue::String("grpc-service-name".into());
        assert_eq!(
            mapping.get(&key).and_then(|v| v.as_str()),
            Some("my-service")
        );
    }

    #[test]
    fn vmess_h2() {
        use base64::Engine;
        let cfg = serde_json::json!({
            "v": "2",
            "ps": "日本-01",
            "add": "jp1.example.com",
            "port": "443",
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "aid": "0",
            "net": "h2",
            "path": "/api",
            "host": "example.com",
            "tls": "tls"
        });
        let b64 = base64::engine::general_purpose::STANDARD.encode(cfg.to_string());
        let node = parse_uri(&format!("vmess://{b64}")).unwrap();
        assert_eq!(node.extra.get("network").and_then(|v| v.as_str()), Some("h2"));
        let h2_opts = node.extra.get("h2-opts").unwrap().as_mapping().unwrap();
        assert_eq!(
            h2_opts
                .get(&YamlValue::String("path".into()))
                .and_then(|v| v.as_str()),
            Some("/api")
        );
        assert_eq!(
            h2_opts
                .get(&YamlValue::String("host".into()))
                .and_then(|v| v.as_str()),
            Some("example.com")
        );
    }

    #[test]
    fn vmess_tcp_no_tls() {
        use base64::Engine;
        let cfg = serde_json::json!({
            "v": "2",
            "ps": "日本-01",
            "add": "jp1.example.com",
            "port": "443",
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "aid": "0",
            "net": "tcp",
            "tls": "none"
        });
        let b64 = base64::engine::general_purpose::STANDARD.encode(cfg.to_string());
        let node = parse_uri(&format!("vmess://{b64}")).unwrap();
        assert!(node.extra.get("network").is_none());
        assert_eq!(node.extra.get("tls").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn vmess_remarks_host_alias() {
        use base64::Engine;
        let cfg = serde_json::json!({
            "v": "2",
            "remarks": "香港-01",
            "add": "hk.example.com",
            "port": "80",
            "id": "uuid",
            "aid": "0",
            "net": "tcp"
        });
        let b64 = base64::engine::general_purpose::STANDARD.encode(cfg.to_string());
        let node = parse_uri(&format!("vmess://{b64}")).unwrap();
        assert_eq!(node.name, "香港-01");
        assert_eq!(node.server, "hk.example.com");
    }

    #[test]
    fn vmess_client_fingerprint_alpn() {
        use base64::Engine;
        let cfg = serde_json::json!({
            "v": "2",
            "ps": "日本-01",
            "add": "jp1.example.com",
            "port": "443",
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "aid": "0",
            "net": "ws",
            "tls": "tls",
            "fp": "chrome",
            "alpn": "h2,http/1.1"
        });
        let b64 = base64::engine::general_purpose::STANDARD.encode(cfg.to_string());
        let node = parse_uri(&format!("vmess://{b64}")).unwrap();
        assert_eq!(
            node.extra
                .get("client-fingerprint")
                .and_then(|v| v.as_str()),
            Some("chrome")
        );
        let alpn = node.extra.get("alpn").unwrap().as_sequence().unwrap();
        assert_eq!(alpn.len(), 2);
        assert_eq!(alpn[0].as_str(), Some("h2"));
        assert_eq!(alpn[1].as_str(), Some("http/1.1"));
    }

    #[test]
    fn trojan_allow_insecure() {
        let uri = "trojan://pass@us1.example.com:443?allowInsecure=1&sni=us1.example.com#美国";
        let node = parse_uri(uri).unwrap();
        assert_eq!(
            node.extra.get("skip-cert-verify").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn trojan_fallback_name_to_hostname() {
        let uri = "trojan://pass@jp1.example.com:443";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node.name, "jp1.example.com");
    }

    #[test]
    fn hysteria2_hy2_prefix() {
        let uri = "hy2://pass@sg1.example.com:8443?insecure=1&alpn=h3,h2#新加坡-Hy2";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node.proxy_type, ProxyType::Hysteria2);
        assert_eq!(node.server, "sg1.example.com");
        assert_eq!(node.port, 8443);
        let alpn = node.extra.get("alpn").unwrap().as_sequence().unwrap();
        assert_eq!(alpn.len(), 2);
        assert_eq!(alpn[0].as_str(), Some("h3"));
        assert_eq!(alpn[1].as_str(), Some("h2"));
    }

    #[test]
    fn hysteria2_up_down() {
        let uri = "hysteria2://pass@us1.example.com:443?up=50&down=150&insecure=1#美国";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node.extra.get("up").and_then(|v| v.as_u64()), Some(50));
        assert_eq!(node.extra.get("down").and_then(|v| v.as_u64()), Some(150));
    }

    #[test]
    fn hysteria2_fallback_name_to_hostname() {
        let uri = "hysteria2://pass@hk1.example.com:443";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node.name, "hk1.example.com");
        assert!(node.extra.get("skip-cert-verify").is_none());
    }

    #[test]
    fn parse_lines_skips_unparseable() {
        let lines = vec![
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#美国".to_string(),
            "unknown://bad-line".to_string(),
            "trojan://pass@sg.example.com:443?sni=sg.example.com#新加坡".to_string(),
        ];
        let nodes = parse_lines(&lines);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].proxy_type, ProxyType::Ss);
        assert_eq!(nodes[1].proxy_type, ProxyType::Trojan);
    }
}
