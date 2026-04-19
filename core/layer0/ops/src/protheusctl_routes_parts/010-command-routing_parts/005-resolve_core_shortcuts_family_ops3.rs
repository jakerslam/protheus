fn resolve_core_shortcuts_family_ops3(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
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
                .map(|v| v.trim().eq_ignore_ascii_case("run"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["ephemeral-run".to_string()];
            if let Some(goal) = rest.get(1) {
                if !goal.starts_with("--") {
                    args.push(format!("--goal={goal}"));
                }
            }
            args.extend(
                rest.iter()
                    .skip(2)
                    .filter(|v| !v.trim().eq_ignore_ascii_case("--ephemeral"))
                    .cloned(),
            );
            Some(Route {
                script_rel: "core://autonomy-controller".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "agent"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("status"))
                .unwrap_or(false)
                && rest
                    .iter()
                    .any(|v| v.trim().eq_ignore_ascii_case("--trunk")) =>
        {
            let mut args = vec!["trunk-status".to_string()];
            args.extend(
                rest.iter()
                    .skip(1)
                    .filter(|v| !v.trim().eq_ignore_ascii_case("--trunk"))
                    .cloned(),
            );
            Some(Route {
                script_rel: "core://autonomy-controller".to_string(),
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
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("stake"))
                .unwrap_or(false)
                && rest.iter().any(|v| v.trim().starts_with("--market"))
            {
                let market = rest
                    .iter()
                    .find_map(|v| v.trim().split_once("--market=").map(|(_, m)| m.to_string()))
                    .unwrap_or_else(|| "unknown".to_string());
                return Some(Route {
                    script_rel: "core://network-protocol".to_string(),
                    args: vec![
                        "stake".to_string(),
                        "--action=stake".to_string(),
                        "--agent=economy:operator".to_string(),
                        "--amount=10".to_string(),
                        format!("--reason=market:{market}"),
                    ],
                    forward_stdin: false,
                });
            }
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
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("join"))
                .unwrap_or(false)
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("hyperspace"))
                    .unwrap_or(false)
            {
                let args = std::iter::once("join-hyperspace".to_string())
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
                    | "contribution"
                    | "consensus"
                    | "rsi-boundary"
                    | "governance-vote"
                    | "join-hyperspace"
                    | "merkle-root"
                    | "emission"
                    | "zk-claim"
                    | "oracle-query"
                    | "truth-weight"
                    | "dashboard"
            ) {
                let args = if rest.is_empty() {
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
        "enterprise" => {
            let args = if rest.is_empty() {
                vec!["dashboard".to_string()]
            } else if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("enable"))
                .unwrap_or(false)
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("bedrock"))
                    .unwrap_or(false)
            {
                std::iter::once("enable-bedrock".to_string())
                    .chain(rest.iter().skip(2).cloned())
                    .collect::<Vec<_>>()
            } else if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("compliance"))
                .unwrap_or(false)
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("export"))
                    .unwrap_or(false)
            {
                std::iter::once("export-compliance".to_string())
                    .chain(rest.iter().skip(2).cloned())
                    .collect::<Vec<_>>()
            } else if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("identity"))
                .unwrap_or(false)
            {
                std::iter::once("identity-surface".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>()
            } else if rest
                .first()
                .map(|v| {
                    v.trim().eq_ignore_ascii_case("scale")
                        || v.trim().eq_ignore_ascii_case("certify-scale")
                })
                .unwrap_or(false)
            {
                let skip = if rest
                    .first()
                    .map(|v| v.trim().eq_ignore_ascii_case("scale"))
                    .unwrap_or(false)
                {
                    1
                } else {
                    0
                };
                std::iter::once("certify-scale".to_string())
                    .chain(rest.iter().skip(skip).cloned())
                    .collect::<Vec<_>>()
            } else if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("moat"))
                .unwrap_or(false)
            {
                match rest
                    .get(1)
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "contrast".to_string())
                    .as_str()
                {
                    "license" => std::iter::once("moat-license".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>(),
                    "launch-sim" | "launch" => std::iter::once("moat-launch-sim".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>(),
                    _ => std::iter::once("moat-contrast".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>(),
                }
            } else if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("genesis"))
                .unwrap_or(false)
            {
                match rest
                    .get(1)
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "truth-gate".to_string())
                    .as_str()
                {
                    "truth-gate" | "gate" => std::iter::once("genesis-truth-gate".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>(),
                    "thin-wrapper-audit" | "thin-wrapper" | "audit" => {
                        std::iter::once("genesis-thin-wrapper-audit".to_string())
                            .chain(rest.iter().skip(2).cloned())
                            .collect::<Vec<_>>()
                    }
                    "doc-freeze" | "freeze" => std::iter::once("genesis-doc-freeze".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>(),
                    "bootstrap" => std::iter::once("genesis-bootstrap".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>(),
                    "installer-sim" | "installer" => {
                        std::iter::once("genesis-installer-sim".to_string())
                            .chain(rest.iter().skip(2).cloned())
                            .collect::<Vec<_>>()
                    }
                    _ => std::iter::once("genesis-truth-gate".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>(),
                }
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://enterprise-hardening".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "moat" => {
            let args = match rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "contrast".to_string())
                .as_str()
            {
                "license" => std::iter::once("moat-license".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "launch-sim" | "launch" => std::iter::once("moat-launch-sim".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "replay" => std::iter::once("replay".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "explore" => std::iter::once("explore".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "ai" => std::iter::once("ai".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "sync" => std::iter::once("sync".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "energy-cert" | "energy" => std::iter::once("energy-cert".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "migrate" => std::iter::once("migrate-ecosystem".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "chaos" => std::iter::once("chaos-run".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "assistant" => std::iter::once("assistant-mode".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                _ => std::iter::once("moat-contrast".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
            };
            Some(Route {
                script_rel: "core://enterprise-hardening".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "explore" => Some(Route {
            script_rel: "core://enterprise-hardening".to_string(),
            args: std::iter::once("explore".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "ai" => Some(Route {
            script_rel: "core://enterprise-hardening".to_string(),
            args: std::iter::once("ai".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "chaos" => Some(Route {
            script_rel: "core://enterprise-hardening".to_string(),
            args: if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("isolate"))
                .unwrap_or(false)
            {
                std::iter::once("chaos-run".to_string())
                    .chain(std::iter::once("--suite=isolate".to_string()))
                    .chain(rest.iter().skip(1).cloned())
                    .collect()
            } else {
                std::iter::once("chaos-run".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect()
            },
            forward_stdin: false,
        }),
        "assistant" => Some(Route {
            script_rel: "core://enterprise-hardening".to_string(),
            args: std::iter::once("assistant-mode".to_string())
                .chain(rest.iter().cloned())
                .collect(),
            forward_stdin: false,
        }),
        "genesis" => {
            let args = match rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "truth-gate".to_string())
                .as_str()
            {
                "truth-gate" | "gate" => std::iter::once("genesis-truth-gate".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "thin-wrapper-audit" | "thin-wrapper" | "audit" => {
                    std::iter::once("genesis-thin-wrapper-audit".to_string())
                        .chain(rest.iter().skip(1).cloned())
                        .collect::<Vec<_>>()
                }
                "doc-freeze" | "freeze" => std::iter::once("genesis-doc-freeze".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "bootstrap" => std::iter::once("genesis-bootstrap".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                "installer-sim" | "installer" => {
                    std::iter::once("genesis-installer-sim".to_string())
                        .chain(rest.iter().skip(1).cloned())
                        .collect::<Vec<_>>()
                }
                _ => std::iter::once("genesis-truth-gate".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
            };
            Some(Route {
                script_rel: "core://enterprise-hardening".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "seed" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let args = match sub.as_str() {
                "deploy" | "ignite" => {
                    let mut out = vec!["deploy".to_string()];
                    let mut skip = 1usize;
                    if let Some(profile) = rest.get(1).map(|v| v.trim().to_ascii_lowercase()) {
                        if profile == "viral" || profile == "immortal" {
                            out.push(format!("--profile={profile}"));
                            skip = 2;
                        }
                    }
                    out.extend(rest.iter().skip(skip).cloned());
                    out
                }
                "monitor" => {
                    let mut out = vec!["monitor".to_string()];
                    out.extend(rest.iter().skip(1).cloned());
                    out
                }
                "status" | "migrate" | "enforce" | "select" | "archive" | "defend" => {
                    if rest.is_empty() {
                        vec!["status".to_string()]
                    } else {
                        rest.to_vec()
                    }
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
                script_rel: "core://seed-protocol".to_string(),
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
            let mut args = vec!["dashboard".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://skills-plane".to_string(),
                args,
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
        "skills" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://skills-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "skill" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let mut args = match sub.as_str() {
                "create" => vec!["create".to_string()],
                "list" => vec!["list".to_string()],
                "dashboard" => vec!["dashboard".to_string()],
                "activate" => vec!["activate".to_string()],
                "install" => vec!["install".to_string()],
                "run" => vec!["run".to_string()],
                "share" => vec!["share".to_string()],
                "gallery" => vec!["gallery".to_string()],
                "load" => vec!["load".to_string()],
                "react" | "react-minimal" | "react_minimal" => vec!["react-minimal".to_string()],
                "tot" | "tot-deliberate" | "tot_deliberate" => vec!["tot-deliberate".to_string()],
                "chain" | "chain-validate" | "chain_validate" => vec!["chain-validate".to_string()],
                "status" => vec!["status".to_string()],
                _ => {
                    let mut out = vec![sub.clone()];
                    out.extend(rest.iter().skip(1).cloned());
                    out
                }
            };
            if sub == "create" {
                let mut forwarded_name = false;
                for row in rest.iter().skip(1) {
                    if row.starts_with("--name=") {
                        args.push(row.clone());
                        forwarded_name = true;
                    } else if row.starts_with("--task=") {
                        args.push(row.replacen("--task=", "--name=", 1));
                        forwarded_name = true;
                    } else if row.starts_with("--") {
                        args.push(row.clone());
                    }
                }
                if !forwarded_name {
                    let name = rest
                        .iter()
                        .skip(1)
                        .filter(|row| !row.starts_with("--"))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !name.trim().is_empty() {
                        args.push(format!("--name={name}"));
                    }
                }
            } else if sub == "load" {
                if let Some(skill) = rest
                    .iter()
                    .skip(1)
                    .find(|row| !row.starts_with("--"))
                    .cloned()
                {
                    args.push(format!("--skill={skill}"));
                }
                args.extend(
                    rest.iter()
                        .skip(1)
                        .filter(|row| row.starts_with("--"))
                        .cloned(),
                );
            } else if !rest.is_empty() {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://skills-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "binary-vuln" | "binvuln" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://binary-vuln-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "scan"
            if rest
                .first()
                .map(|v| {
                    let s = v.trim().to_ascii_lowercase();
                    s == "binary" || s == "firmware" || s == "uefi" || s == "ba2"
                })
                .unwrap_or(false) =>
        {
            let mut args = vec!["scan".to_string(), "--dx-source=scan-binary".to_string()];
            if let Some(input) = rest.get(1) {
                if !input.starts_with("--") {
                    args.push(format!("--input={input}"));
                }
            }
            args.extend(
                rest.iter()
                    .skip(2)
                    .filter(|row| row.starts_with("--"))
                    .cloned(),
            );
            Some(Route {
                script_rel: "core://binary-vuln-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "browser" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "session-start".to_string());
            let mut args = match sub.as_str() {
                "start" | "open" | "session-start" => vec!["session-start".to_string()],
                "join" => vec!["session-control".to_string(), "--op=join".to_string()],
                "handoff" => vec!["session-control".to_string(), "--op=handoff".to_string()],
                "leave" => vec!["session-control".to_string(), "--op=leave".to_string()],
                "control" | "session-control" => vec!["session-control".to_string()],
                "automate" => vec!["automate".to_string()],
                "privacy" | "privacy-guard" => vec!["privacy-guard".to_string()],
                "snapshot" => vec!["snapshot".to_string()],
                "screenshot" => vec!["screenshot".to_string()],
                "action-policy" => vec!["action-policy".to_string()],
                "auth-save" => vec!["auth-save".to_string()],
                "auth-login" => vec!["auth-login".to_string()],
                "native" => vec!["native".to_string()],
                "status" => vec!["status".to_string()],
                _ => vec!["session-start".to_string()],
            };
            if !rest.is_empty() {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://vbrowser-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "hand" | "hands" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let scheduled_mode = matches!(sub.as_str(), "enable" | "scheduled" | "dashboard")
                && (sub != "enable"
                    || rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("scheduled"))
                        .unwrap_or(false));
            let mut args = match sub.as_str() {
                "enable"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("scheduled"))
                        .unwrap_or(false) =>
                {
                    vec!["scheduled-hands".to_string(), "--op=enable".to_string()]
                }
                "scheduled" => vec!["scheduled-hands".to_string(), "--op=run".to_string()],
                "dashboard" => vec!["scheduled-hands".to_string(), "--op=dashboard".to_string()],
                "new" => vec!["hand-new".to_string()],
                "schedule" | "cycle" | "run" => vec!["hand-cycle".to_string()],
                "status" => vec!["hand-status".to_string()],
                "memory-page" | "memory" => vec!["hand-memory-page".to_string()],
                "wasm-task" | "wasm" => vec!["hand-wasm-task".to_string()],
                _ => vec!["hand-status".to_string()],
            };
            if !rest.is_empty() {
                let skip = if sub == "enable"
                    && rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("scheduled"))
                        .unwrap_or(false)
                {
                    2
                } else {
                    1
                };
                args.extend(rest.iter().skip(skip).cloned());
            }
            Some(Route {
                script_rel: if scheduled_mode {
                    "core://assimilation-controller".to_string()
                } else {
                    "core://autonomy-controller".to_string()
                },
                args,
                forward_stdin: false,
            })
        }
        "oracle" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "query".to_string());
            let args = match sub.as_str() {
                "query" => {
                    let provider = rest
                        .iter()
                        .find_map(|v| {
                            v.trim()
                                .split_once("--provider=")
                                .map(|(_, p)| p.to_string())
                        })
                        .or_else(|| {
                            rest.iter()
                                .skip(1)
                                .find(|v| !v.trim().starts_with("--"))
                                .cloned()
                        })
                        .unwrap_or_else(|| "polymarket".to_string());
                    let event = rest
                        .iter()
                        .find_map(|v| v.trim().split_once("--event=").map(|(_, e)| e.to_string()))
                        .or_else(|| {
                            rest.iter()
                                .skip(2)
                                .find(|v| !v.trim().starts_with("--"))
                                .cloned()
                        })
                        .unwrap_or_else(|| "default-event".to_string());
                    vec![
                        "oracle-query".to_string(),
                        format!("--provider={provider}"),
                        format!("--event={event}"),
                    ]
                }
                _ => vec!["oracle-query".to_string()],
            };
            Some(Route {
                script_rel: "core://network-protocol".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "provider" | "providers" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let provider = rest
                .iter()
                .find_map(|v| {
                    v.trim()
                        .split_once("--provider=")
                        .map(|(_, p)| p.to_string())
                })
                .or_else(|| {
                    if matches!(sub.as_str(), "switch" | "set") {
                        rest.get(1).map(|v| v.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let model = rest
                .iter()
                .find_map(|v| v.trim().split_once("--model=").map(|(_, m)| m.to_string()))
                .or_else(|| {
                    if matches!(sub.as_str(), "switch" | "set") {
                        rest.get(2).map(|v| v.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let args = if matches!(sub.as_str(), "switch" | "set") {
                let mut rows = vec![
                    "app-plane".to_string(),
                    "switch-provider".to_string(),
                    "--app=chat-ui".to_string(),
                ];
                if !provider.is_empty() {
                    rows.push(format!("--provider={provider}"));
                }
                if !model.is_empty() {
                    rows.push(format!("--model={model}"));
                }
                rows
            } else {
                vec![
                    "app-plane".to_string(),
                    "status".to_string(),
                    "--app=chat-ui".to_string(),
                ]
            };
            Some(Route {
                script_rel: "core://ops-main".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "truth"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("weight"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["truth-weight".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://network-protocol".to_string(),
                args,
                forward_stdin: false,
            })
        }
        _ => resolve_core_shortcuts_family_misc(cmd, rest),
    }
}
