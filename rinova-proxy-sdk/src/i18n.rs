use std::collections::HashMap;
use std::sync::OnceLock;

const EN_JSON: &str = include_str!("../locales/en.json");
const ZH_JSON: &str = include_str!("../locales/zh.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

static LANG: OnceLock<Lang> = OnceLock::new();
static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
static ZH: OnceLock<HashMap<String, String>> = OnceLock::new();

fn detect_lang() -> Lang {
    let lang = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .unwrap_or_default();
    if lang.starts_with("zh") {
        Lang::Zh
    } else {
        Lang::En
    }
}

fn load_locale(json: &str) -> HashMap<String, String> {
    serde_json::from_str(json).unwrap_or_default()
}

pub fn get_lang() -> Lang {
    *LANG.get_or_init(detect_lang)
}

pub fn t(key: &str, params: &[(&str, &str)]) -> String {
    t_with_fallback(key, params, None)
}

pub fn t_with_fallback(key: &str, params: &[(&str, &str)], fallback: Option<&str>) -> String {
    let en = EN.get_or_init(|| load_locale(EN_JSON));
    let zh = ZH.get_or_init(|| load_locale(ZH_JSON));

    let mut msg = match get_lang() {
        Lang::Zh => zh.get(key).or_else(|| en.get(key)),
        Lang::En => en.get(key),
    }
    .cloned()
    .or_else(|| fallback.map(str::to_string))
    .unwrap_or_else(|| key.to_string());

    for (k, v) in params {
        msg = msg.replace(&format!("{{{k}}}"), v);
    }

    msg
}

pub fn t_simple(key: &str) -> String {
    t(key, &[])
}
