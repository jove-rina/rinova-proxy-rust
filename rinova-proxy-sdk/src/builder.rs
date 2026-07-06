use crate::types::{ClashConfig, ProxyGroup, ProxyNode, RuleMode};

const BUILTIN_RULES: &[&str] = &[
    "DOMAIN-SUFFIX,ad.com,🛑 广告拦截",
    "DOMAIN-SUFFIX,doubleclick.net,🛑 广告拦截",
    "DOMAIN-SUFFIX,google.com,🌍 国外网站",
    "DOMAIN-SUFFIX,youtube.com,🌍 国外网站",
    "DOMAIN-SUFFIX,github.com,🌍 国外网站",
    "DOMAIN-SUFFIX,twitter.com,🌍 国外网站",
    "DOMAIN-SUFFIX,x.com,🌍 国外网站",
    "DOMAIN-SUFFIX,telegram.org,🌍 国外网站",
    "DOMAIN-SUFFIX,steampowered.com,🌍 国外网站",
    "DOMAIN-SUFFIX,openai.com,🌍 国外网站",
    "DOMAIN-SUFFIX,anthropic.com,🌍 国外网站",
    "DOMAIN-SUFFIX,claude.ai,🌍 国外网站",
    "DOMAIN-SUFFIX,cursor.com,🌍 国外网站",
    "DOMAIN-SUFFIX,cloudflare.com,🌍 国外网站",
    "DOMAIN-SUFFIX,netflix.com,🌍 国外网站",
    "DOMAIN-SUFFIX,spotify.com,🌍 国外网站",
    "DOMAIN-SUFFIX,disneyplus.com,🌍 国外网站",
    "DOMAIN-SUFFIX,cn,🇨🇳 国内网站",
    "DOMAIN-SUFFIX,baidu.com,🇨🇳 国内网站",
    "DOMAIN-SUFFIX,qq.com,🇨🇳 国内网站",
    "DOMAIN-SUFFIX,weixin.com,🇨🇳 国内网站",
    "DOMAIN-SUFFIX,alipay.com,🇨🇳 国内网站",
    "GEOIP,CN,🎯 直连",
    "MATCH,🌍 国外网站",
];

const EXT_RULES: &[&str] = &[
    "RULE-SET,https://raw.githubusercontent.com/ACL4SSR/ACL4SSR/master/Clash/BanEasyAD.list,🛑 广告拦截",
    "RULE-SET,https://raw.githubusercontent.com/ACL4SSR/ACL4SSR/master/Clash/ProxyMedia.list,🌍 国外网站",
    "RULE-SET,https://raw.githubusercontent.com/ACL4SSR/ACL4SSR/master/Clash/ChinaDomain.list,🇨🇳 国内网站",
    "GEOIP,CN,🎯 直连",
    "MATCH,🌍 国外网站",
];

pub fn build_config(nodes: &[ProxyNode], rule_mode: RuleMode) -> ClashConfig {
    let names: Vec<String> = nodes.iter().map(|n| n.name.clone()).collect();
    let mut foreign_proxies = vec![
        "🚀 节点选择".to_string(),
        "♻️ 自动选择".to_string(),
        "🎯 直连".to_string(),
        "DIRECT".to_string(),
    ];
    foreign_proxies.extend(names.clone());

    let rules = match rule_mode {
        RuleMode::External => EXT_RULES.iter().map(|s| (*s).to_string()).collect(),
        RuleMode::Builtin => BUILTIN_RULES.iter().map(|s| (*s).to_string()).collect(),
    };

    ClashConfig {
        port: 7890,
        socks_port: 7891,
        allow_lan: true,
        mode: "Rule".to_string(),
        log_level: "info".to_string(),
        external_controller: "127.0.0.1:9090".to_string(),
        proxies: nodes.to_vec(),
        proxy_groups: vec![
            ProxyGroup {
                name: "🚀 节点选择".to_string(),
                group_type: "select".to_string(),
                proxies: {
                    let mut p = vec![
                        "♻️ 自动选择".to_string(),
                        "🎯 直连".to_string(),
                        "DIRECT".to_string(),
                    ];
                    p.extend(names.clone());
                    p
                },
                url: None,
                interval: None,
            },
            ProxyGroup {
                name: "♻️ 自动选择".to_string(),
                group_type: "url-test".to_string(),
                proxies: names.clone(),
                url: Some("http://www.gstatic.com/generate_204".to_string()),
                interval: Some(300),
            },
            ProxyGroup {
                name: "🎯 直连".to_string(),
                group_type: "select".to_string(),
                proxies: {
                    let mut p = vec!["DIRECT".to_string()];
                    p.extend(names.clone());
                    p
                },
                url: None,
                interval: None,
            },
            ProxyGroup {
                name: "🛑 广告拦截".to_string(),
                group_type: "select".to_string(),
                proxies: {
                    let mut p = vec!["REJECT".to_string(), "DIRECT".to_string()];
                    p.extend(names.clone());
                    p
                },
                url: None,
                interval: None,
            },
            ProxyGroup {
                name: "🌍 国外网站".to_string(),
                group_type: "select".to_string(),
                proxies: foreign_proxies,
                url: None,
                interval: None,
            },
            ProxyGroup {
                name: "🇨🇳 国内网站".to_string(),
                group_type: "select".to_string(),
                proxies: {
                    let mut p = vec!["🎯 直连".to_string()];
                    p.extend(names);
                    p
                },
                url: None,
                interval: None,
            },
        ],
        rules,
    }
}

pub fn to_yaml(config: &ClashConfig) -> crate::Result<String> {
    Ok(serde_yaml::to_string(config)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProxyType;

    fn sample_nodes() -> Vec<ProxyNode> {
        vec![
            ProxyNode::new("节点-A".into(), ProxyType::Ss, "1.2.3.4".into(), 443),
            ProxyNode::new("节点-B".into(), ProxyType::Ss, "5.6.7.8".into(), 443),
        ]
    }

    #[test]
    fn foreign_group_follows_select() {
        let config = build_config(&sample_nodes(), RuleMode::Builtin);
        let foreign = config
            .proxy_groups
            .iter()
            .find(|g| g.name == "🌍 国外网站")
            .unwrap();
        assert_eq!(foreign.proxies[0], "🚀 节点选择");
    }

    #[test]
    fn select_group_contains_all_nodes() {
        let config = build_config(&sample_nodes(), RuleMode::Builtin);
        let select = config
            .proxy_groups
            .iter()
            .find(|g| g.name == "🚀 节点选择")
            .unwrap();
        assert!(select.proxies.contains(&"节点-A".to_string()));
        assert!(select.proxies.contains(&"节点-B".to_string()));
    }

    #[test]
    fn external_rules_use_rule_set() {
        let config = build_config(&sample_nodes(), RuleMode::External);
        assert!(config.rules[0].starts_with("RULE-SET"));
    }
}
