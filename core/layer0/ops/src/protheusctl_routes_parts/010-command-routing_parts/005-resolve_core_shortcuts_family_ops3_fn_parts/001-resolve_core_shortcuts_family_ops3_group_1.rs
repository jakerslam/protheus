fn resolve_core_shortcuts_family_ops3_group_1(cmd: &str, rest: &[String]) -> Option<Route> {
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
        _ => None,
    }
}
