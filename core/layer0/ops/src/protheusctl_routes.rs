use super::Route;

pub(super) fn resolve_core_shortcuts(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "rag" => Some(Route {
            script_rel: "core://rag".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "memory" => {
            let mut args = vec!["memory".to_string()];
            if rest.is_empty() {
                args.push("status".to_string());
            } else {
                args.extend(rest.iter().cloned());
            }
            Some(Route {
                script_rel: "core://rag".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "chat"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("nano"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["chat".to_string(), "nano".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://rag".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "train"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("nano"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["train".to_string(), "nano".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://rag".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "nano" => {
            let mut args = vec!["nano".to_string()];
            if rest.is_empty() {
                args.push("chat".to_string());
            } else {
                args.extend(rest.iter().cloned());
            }
            Some(Route {
                script_rel: "core://rag".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "chat"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("with"))
                .unwrap_or(false)
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("files"))
                    .unwrap_or(false) =>
        {
            let mut args = vec!["chat".to_string()];
            args.extend(rest.iter().skip(2).cloned());
            Some(Route {
                script_rel: "core://rag".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "eval" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "benchmark-neuralavb".to_string());
            let args = match sub.as_str() {
                "enable"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("neuralavb"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("enable-neuralavb".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                "experiment"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("loop"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("experiment-loop".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                "benchmark" => std::iter::once("benchmark-neuralavb".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                _ => {
                    if rest.is_empty() {
                        vec!["benchmark-neuralavb".to_string()]
                    } else {
                        rest.to_vec()
                    }
                }
            };
            Some(Route {
                script_rel: "core://ab-lane-eval".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "experiment"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("loop"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["experiment-loop".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://ab-lane-eval".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "model" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            if sub == "buy"
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("credits"))
                    .unwrap_or(false)
            {
                let args = std::iter::once("buy-credits".to_string())
                    .chain(rest.iter().skip(2).cloned())
                    .collect::<Vec<_>>();
                return Some(Route {
                    script_rel: "core://intelligence-nexus".to_string(),
                    args,
                    forward_stdin: false,
                });
            }
            let args = match sub.as_str() {
                "optimize"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("minimax"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("optimize".to_string())
                        .chain(std::iter::once("--profile=minimax".to_string()))
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                "use"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("cheap"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("optimize".to_string())
                        .chain(std::iter::once("--profile=minimax".to_string()))
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                _ => {
                    if rest.is_empty() {
                        vec!["status".to_string()]
                    } else {
                        rest.to_vec()
                    }
                }
            };
            Some(Route {
                script_rel: "core://model-router".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "keys" => {
            let args = if rest.is_empty() {
                vec!["open".to_string()]
            } else {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "open".to_string());
                match sub.as_str() {
                    "add" => std::iter::once("add-key".to_string())
                        .chain(rest.iter().skip(1).cloned())
                        .collect::<Vec<_>>(),
                    "rotate" => std::iter::once("rotate-key".to_string())
                        .chain(rest.iter().skip(1).cloned())
                        .collect::<Vec<_>>(),
                    "revoke" | "remove" => std::iter::once("revoke-key".to_string())
                        .chain(rest.iter().skip(1).cloned())
                        .collect::<Vec<_>>(),
                    _ => rest.to_vec(),
                }
            };
            Some(Route {
                script_rel: "core://intelligence-nexus".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "graph" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://graph-toolkit".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "blobs" | "blob" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://binary-blob-runtime".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "directives" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://directive-kernel".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "prime"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("sign"))
                .unwrap_or(false) =>
        {
            let args = std::iter::once("prime-sign".to_string())
                .chain(rest.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(Route {
                script_rel: "core://directive-kernel".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "organism" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://organism-layer".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "rsi" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://rsi-ignition".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "veto" => {
            let mut args = vec![
                "compliance-check".to_string(),
                "--action=veto".to_string(),
                "--allow=0".to_string(),
            ];
            args.extend(rest.iter().cloned());
            Some(Route {
                script_rel: "core://directive-kernel".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "agent"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("reset"))
                .unwrap_or(false) =>
        {
            let args = std::iter::once("reset-agent".to_string())
                .chain(rest.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(Route {
                script_rel: "core://model-router".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "agent"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("debate"))
                .unwrap_or(false)
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("bullbear"))
                    .unwrap_or(false) =>
        {
            let mut args = vec!["debate-bullbear".to_string()];
            args.extend(rest.iter().skip(2).cloned());
            Some(Route {
                script_rel: "core://llm-economy-organ".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "economy" => {
            let args = if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("upgrade"))
                .unwrap_or(false)
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("trading-hand"))
                    .unwrap_or(false)
            {
                let mut args = vec!["upgrade-trading-hand".to_string()];
                args.extend(rest.iter().skip(2).cloned());
                args
            } else if rest.is_empty() {
                vec!["dashboard".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://llm-economy-organ".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "network" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("ignite"))
                .unwrap_or(false)
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("bitcoin"))
                    .unwrap_or(false)
            {
                let args = std::iter::once("ignite-bitcoin".to_string())
                    .chain(rest.iter().skip(2).cloned())
                    .collect::<Vec<_>>();
                return Some(Route {
                    script_rel: "core://network-protocol".to_string(),
                    args,
                    forward_stdin: false,
                });
            }
            if matches!(
                sub.as_str(),
                "status"
                    | "stake"
                    | "reward"
                    | "slash"
                    | "merkle-root"
                    | "emission"
                    | "zk-claim"
                    | "dashboard"
            ) {
                let args = if sub == "dashboard" {
                    vec!["status".to_string()]
                } else if rest.is_empty() {
                    vec!["status".to_string()]
                } else {
                    rest.to_vec()
                };
                return Some(Route {
                    script_rel: "core://network-protocol".to_string(),
                    args,
                    forward_stdin: false,
                });
            }
            let args = if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("join"))
                .unwrap_or(false)
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("hyperspace"))
                    .unwrap_or(false)
            {
                let mut args = vec![
                    "discover".to_string(),
                    "--profile=hyperspace".to_string(),
                    "--apply=1".to_string(),
                ];
                args.extend(rest.iter().skip(2).cloned());
                args
            } else if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("dashboard"))
                .unwrap_or(false)
            {
                let mut args = vec!["status".to_string(), "--dashboard=1".to_string()];
                args.extend(rest.iter().skip(1).cloned());
                args
            } else if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://p2p-gossip-seed".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "compute"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("share"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["compute-proof".to_string(), "--share=1".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://p2p-gossip-seed".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "skills"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("enable"))
                .unwrap_or(false) =>
        {
            let mode = rest
                .get(1)
                .cloned()
                .unwrap_or_else(|| "perplexity-mode".to_string());
            let mut args = vec!["skills-enable".to_string(), mode];
            args.extend(rest.iter().skip(2).cloned());
            Some(Route {
                script_rel: "core://assimilation-controller".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "skills"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("dashboard"))
                .unwrap_or(false) =>
        {
            Some(Route {
                script_rel: "core://assimilation-controller".to_string(),
                args: vec!["skills-dashboard".to_string()],
                forward_stdin: false,
            })
        }
        "skills"
            if rest
                .first()
                .map(|v| {
                    let s = v.trim().to_ascii_lowercase();
                    s == "spawn" || s == "spawn-subagents"
                })
                .unwrap_or(false) =>
        {
            let mut args = vec!["skills-spawn-subagents".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://assimilation-controller".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "skills"
            if rest
                .first()
                .map(|v| {
                    let s = v.trim().to_ascii_lowercase();
                    s == "computer-use" || s == "hands"
                })
                .unwrap_or(false) =>
        {
            let mut args = vec!["skills-computer-use".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://assimilation-controller".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "skill"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("create"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["skill-create".to_string()];
            let mut forwarded = false;
            for row in rest.iter().skip(1) {
                if row.starts_with("--task=") {
                    args.push(row.clone());
                    forwarded = true;
                }
            }
            if !forwarded {
                let task = rest
                    .iter()
                    .skip(1)
                    .filter(|row| !row.starts_with("--"))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ");
                if !task.trim().is_empty() {
                    args.push(format!("--task={task}"));
                }
            }
            Some(Route {
                script_rel: "core://assimilation-controller".to_string(),
                args,
                forward_stdin: false,
            })
        }
        _ => None,
    }
}
