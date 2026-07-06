use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
use rinova_proxy_sdk::{
    convert, get_lang, start_server, t, ConvertOptions, Lang, RuleMode, ServerOptions,
};
use serde_yaml::Value;
use url::Url;

struct Cli {
    url: Option<String>,
    output: String,
    port: Option<u16>,
    interval: u64,
    rules: String,
    merge: String,
}

fn build_command() -> Command {
    let (
        about,
        url_help,
        output_help,
        port_help,
        interval_help,
        rules_help,
        merge_help,
    ) = if get_lang() == Lang::Zh {
        (
            "代理订阅链接 → Clash 配置文件",
            "代理订阅链接",
            "输出文件路径",
            "服务模式：监听端口（默认 25500）",
            "服务模式：刷新间隔（分钟）",
            "规则模式: builtin（内置）| external（外部）",
            "合并到现有配置",
        )
    } else {
        (
            "Proxy subscription → Clash config",
            "Proxy subscription URL",
            "output path",
            "serve mode: HTTP port (default 25500)",
            "serve mode: refresh interval (minutes)",
            "rule mode: builtin | external",
            "merge into existing Clash config",
        )
    };

    Command::new("proxy-cli")
        .version("1.0.0")
        .about(about)
        .arg(
            Arg::new("url")
                .short('u')
                .long("url")
                .help(url_help),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help(output_help)
                .default_value("")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help(port_help)
                .value_parser(clap::value_parser!(u16)),
        )
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .help(interval_help)
                .default_value("60")
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            Arg::new("rules")
                .long("rules")
                .help(rules_help)
                .default_value("builtin")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("merge")
                .long("merge")
                .help(merge_help)
                .default_value("")
                .action(ArgAction::Set),
        )
}

fn parse_cli() -> Cli {
    let matches = build_command().get_matches();
    Cli {
        url: matches.get_one::<String>("url").cloned(),
        output: matches
            .get_one::<String>("output")
            .cloned()
            .unwrap_or_default(),
        port: matches.get_one::<u16>("port").copied(),
        interval: *matches.get_one::<u64>("interval").unwrap_or(&60),
        rules: matches
            .get_one::<String>("rules")
            .cloned()
            .unwrap_or_else(|| "builtin".to_string()),
        merge: matches
            .get_one::<String>("merge")
            .cloned()
            .unwrap_or_default(),
    }
}

fn mask_url(url: &str) -> String {
    if let Ok(original) = Url::parse(url) {
        let pairs: Vec<(String, String)> = original
            .query_pairs()
            .map(|(k, _)| (k.into_owned(), "***".to_string()))
            .collect();
        if pairs.is_empty() {
            return url.to_string();
        }
        let mut masked = original.clone();
        let query = pairs
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        masked.set_query(Some(&query));
        return masked.to_string();
    }

    let lower = url.to_lowercase();
    let mut result = url.to_string();
    for pat in ["token", "key", "pass", "id", "service"] {
        if let Some(idx) = lower.find(&format!("{pat}=")) {
            let start = idx + pat.len() + 1;
            if let Some(end) = result[start..].find('&') {
                result.replace_range(start..start + end, "***");
            } else if start <= result.len() {
                result.truncate(start);
                result.push_str("***");
            }
        }
    }
    result
}

const BUILTIN_GROUP_MEMBERS: &[&str] = &[
    "🎯 直连",
    "♻️ 自动选择",
    "🚀 节点选择",
    "🌍 国外网站",
    "🇨🇳 国内网站",
    "🛑 广告拦截",
    "DIRECT",
    "REJECT",
    "PASS",
];

fn parse_rule_mode(rules: &str) -> RuleMode {
    if rules == "external" {
        RuleMode::External
    } else {
        RuleMode::Builtin
    }
}

async fn wait_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
    }
}

async fn run_convert(
    url: &str,
    output: &str,
    rules: &str,
    merge: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 {} {}", t("fetching", &[]), mask_url(url));
    let rule_mode = parse_rule_mode(rules);
    let result = convert(
        url,
        Some(ConvertOptions {
            rules: rule_mode,
            deduplicate: true,
        }),
    )
    .await?;

    println!(
        "✅ {}",
        t("parsed", &[("count", &result.nodes.len().to_string())])
    );
    for (i, node) in result.nodes.iter().enumerate() {
        println!(
            "   {}",
            t(
                "node_list",
                &[
                    ("index", &(i + 1).to_string()),
                    ("type", node.proxy_type.as_str()),
                    ("server", &node.server),
                    ("port", &node.port.to_string()),
                    ("name", &node.name),
                ],
            )
        );
    }

    let new_node_names: HashSet<String> = result.nodes.iter().map(|n| n.name.clone()).collect();

    if !merge.is_empty() && Path::new(merge).exists() {
        let existing_raw = fs::read_to_string(merge)?;
        let mut existing: Value = serde_yaml::from_str(&existing_raw)?;
        if let Value::Mapping(map) = &mut existing {
            map.insert(
                Value::String("proxies".into()),
                serde_yaml::to_value(&result.nodes)?,
            );

            if let Some(Value::Sequence(groups)) = map.get_mut(Value::String("proxy-groups".into()))
            {
                for group in groups {
                    if let Value::Mapping(group_map) = group {
                        let group_name = group_map
                            .get(Value::String("name".into()))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if let Some(Value::Sequence(proxies)) =
                            group_map.get_mut(Value::String("proxies".into()))
                        {
                            for proxy in proxies {
                                if let Value::String(name) = proxy {
                                    if BUILTIN_GROUP_MEMBERS.contains(&name.as_str()) {
                                        continue;
                                    }
                                    if new_node_names.contains(name) {
                                        continue;
                                    }
                                    let fallback = result
                                        .nodes
                                        .first()
                                        .map(|n| n.name.clone())
                                        .unwrap_or_else(|| name.clone());
                                    eprintln!(
                                        "  ⚠️  {}",
                                        t(
                                            "group_fallback",
                                            &[
                                                ("group", &group_name),
                                                ("name", name),
                                                ("fallback", &fallback),
                                            ],
                                        )
                                    );
                                    *name = fallback;
                                }
                            }
                        }
                    }
                }
            }
        }

        let merged_yaml = serde_yaml::to_string(&existing)?;
        fs::write(merge, merged_yaml)?;
        println!("\n📝 {}", t("merged", &[("path", merge)]));
    } else {
        let output_path = if output.is_empty() {
            std::env::current_dir()?.join("clash-config.yaml")
        } else {
            PathBuf::from(output)
        };

        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(&output_path, result.yaml)?;
        println!(
            "\n✅ {}",
            t(
                "config_written",
                &[("path", &output_path.display().to_string())]
            )
        );
        println!(
            "📊 {}",
            t(
                "config_summary",
                &[
                    ("nodes", &result.nodes.len().to_string()),
                    ("groups", &result.config.proxy_groups.len().to_string()),
                ],
            )
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let mut cli = parse_cli();
    cli.rules = cli.rules.to_lowercase();

    if let Some(port) = cli.port {
        let url = match cli.url {
            Some(url) => url,
            None => {
                eprintln!("❌ {}", t("mode_requires_url", &[]));
                build_command().print_help().ok();
                std::process::exit(1);
            }
        };

        let handle = start_server(ServerOptions {
            url,
            port,
            interval_min: cli.interval,
            rule_mode: parse_rule_mode(&cli.rules),
        })
        .await
        .unwrap_or_else(|err| {
            eprintln!("❌ {err}");
            std::process::exit(1);
        });

        wait_shutdown_signal().await;
        println!("\n🛑 {}", t("shutting_down", &[]));
        handle.shutdown().await;
        return;
    }

    if let Some(url) = cli.url {
        if let Err(err) = run_convert(&url, &cli.output, &cli.rules, &cli.merge).await {
            eprintln!("❌ {err}");
            std::process::exit(1);
        }
    } else {
        build_command().print_help().ok();
    }
}
