use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    #[serde(rename = "ss")]
    Ss,
    #[serde(rename = "vmess")]
    Vmess,
    #[serde(rename = "trojan")]
    Trojan,
    #[serde(rename = "hysteria2")]
    Hysteria2,
}

impl ProxyType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ss => "ss",
            Self::Vmess => "vmess",
            Self::Trojan => "trojan",
            Self::Hysteria2 => "hysteria2",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    pub server: String,
    pub port: u16,
    #[serde(flatten, skip_serializing_if = "IndexMap::is_empty")]
    pub extra: IndexMap<String, Value>,
}

impl ProxyNode {
    pub fn new(name: String, proxy_type: ProxyType, server: String, port: u16) -> Self {
        Self {
            name,
            proxy_type,
            server,
            port,
            extra: IndexMap::new(),
        }
    }

    pub fn set_str(&mut self, key: &str, value: impl Into<String>) {
        self.extra.insert(key.to_string(), Value::String(value.into()));
    }

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.extra.insert(key.to_string(), Value::Bool(value));
    }

    pub fn set_u64(&mut self, key: &str, value: u64) {
        self.extra.insert(key.to_string(), Value::Number(value.into()));
    }

    pub fn set_value(&mut self, key: &str, value: Value) {
        self.extra.insert(key.to_string(), value);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashConfig {
    pub port: u16,
    #[serde(rename = "socks-port")]
    pub socks_port: u16,
    #[serde(rename = "allow-lan")]
    pub allow_lan: bool,
    pub mode: String,
    #[serde(rename = "log-level")]
    pub log_level: String,
    #[serde(rename = "external-controller")]
    pub external_controller: String,
    pub proxies: Vec<ProxyNode>,
    #[serde(rename = "proxy-groups")]
    pub proxy_groups: Vec<ProxyGroup>,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuleMode {
    #[default]
    Builtin,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvertOptions {
    pub rules: RuleMode,
    pub deduplicate: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            rules: RuleMode::Builtin,
            deduplicate: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub config: ClashConfig,
    pub yaml: String,
    pub nodes: Vec<ProxyNode>,
}
