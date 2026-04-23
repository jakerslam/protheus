use super::Route;

pub(super) fn resolve_plane_shortcuts(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "agency" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let mut args = match sub.as_str() {
                "create" | "create-shadow" => vec!["create-shadow".to_string()],
                "topology" | "division-topology" => vec!["topology".to_string()],
                "orchestrate" => vec!["orchestrate".to_string()],
                "workflow" | "workflow-bind" => vec!["workflow-bind".to_string()],
                "status" => vec!["status".to_string()],
                _ => vec![sub],
            };
            if !rest.is_empty() {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://agency-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "team" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "dashboard".to_string());
            let mut args = match sub.as_str() {
                "dashboard" => vec!["dashboard".to_string()],
                "launch" | "launch-role" => vec!["launch-role".to_string()],
                "schedule" => vec!["schedule".to_string()],
                "continuity" => vec!["continuity".to_string()],
                "status" => vec!["status".to_string()],
                _ => vec![sub],
            };
            if !rest.is_empty() {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://collab-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "company" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let mut args = match sub.as_str() {
                "orchestrate" | "orchestrate-agency" => vec!["orchestrate-agency".to_string()],
                "budget" | "budget-enforce" => vec!["budget-enforce".to_string()],
                "ticket" => vec!["ticket".to_string()],
                "heartbeat" => vec!["heartbeat".to_string()],
                "status" => vec!["status".to_string()],
                _ => vec![sub],
            };
            if !rest.is_empty() {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://company-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "ticket" => {
            let mut args = vec!["ticket".to_string()];
            args.extend(rest.iter().cloned());
            Some(Route {
                script_rel: "core://company-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "heartbeat" => {
            let mut args = vec!["heartbeat".to_string()];
            args.extend(rest.iter().cloned());
            Some(Route {
                script_rel: "core://company-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "substrate" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let enable_biological = sub == "enable"
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("biological"))
                    .unwrap_or(false);
            let mut args = match sub.as_str() {
                "capture" | "csi-capture" => vec!["csi-capture".to_string()],
                "module" | "csi-module" => vec!["csi-module".to_string()],
                "embedded" | "csi-embedded-profile" => vec!["csi-embedded-profile".to_string()],
                "policy" | "csi-policy" => vec!["csi-policy".to_string()],
                "eye" | "eye-bind" => vec!["eye-bind".to_string()],
                "bio-interface" => vec!["bio-interface".to_string()],
                "bio-feedback" => vec!["bio-feedback".to_string()],
                "bio-adapter-template" => vec!["bio-adapter-template".to_string()],
                "bioethics" | "bioethics-policy" => vec!["bioethics-policy".to_string()],
                "enable" if enable_biological => {
                    vec!["bio-enable".to_string(), "--mode=biological".to_string()]
                }
                "bio-enable" => vec!["bio-enable".to_string()],
                "status" => vec!["status".to_string()],
                _ => vec![sub.clone()],
            };
            if !rest.is_empty() {
                if enable_biological {
                    args.extend(rest.iter().skip(2).cloned());
                } else {
                    args.extend(rest.iter().skip(1).cloned());
                }
            }
            Some(Route {
                script_rel: "core://substrate-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "observability" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let enable_acp_provenance = sub == "enable"
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("acp-provenance"))
                    .unwrap_or(false);
            let mut args = match sub.as_str() {
                "monitor" => vec!["monitor".to_string()],
                "workflow" => vec!["workflow".to_string()],
                "incident" => vec!["incident".to_string()],
                "selfhost" => vec!["selfhost".to_string()],
                "deploy" => vec!["selfhost".to_string(), "--op=deploy".to_string()],
                "acp-provenance" => vec!["acp-provenance".to_string()],
                "trace" => vec!["acp-provenance".to_string(), "--op=trace".to_string()],
                "debug" => vec!["acp-provenance".to_string(), "--op=debug".to_string()],
                "enable" if enable_acp_provenance => {
                    vec!["acp-provenance".to_string(), "--op=enable".to_string()]
                }
                "status" => vec!["status".to_string()],
                _ => vec![sub],
            };
            if !rest.is_empty() {
                if enable_acp_provenance {
                    args.extend(rest.iter().skip(2).cloned());
                } else {
                    args.extend(rest.iter().skip(1).cloned());
                }
            }
            Some(Route {
                script_rel: "core://observability-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "persist" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let mut args = match sub.as_str() {
                "schedule" => vec!["schedule".to_string()],
                "mobile" | "mobile-cockpit" => vec!["mobile-cockpit".to_string()],
                "mobile-daemon" => vec!["mobile-daemon".to_string()],
                "continuity" => vec!["continuity".to_string()],
                "connector" => vec!["connector".to_string()],
                "cowork" | "co-work" => vec!["cowork".to_string()],
                "status" => vec!["status".to_string()],
                _ => vec![sub],
            };
            if !rest.is_empty() {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://persist-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "connector" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "list".to_string());
            let mut args = vec!["connector".to_string(), format!("--op={sub}")];
            if let Some(provider) = rest.get(1) {
                if !provider.starts_with("--") {
                    args.push(format!(
                        "--provider={}",
                        provider.trim().to_ascii_lowercase()
                    ));
                    args.extend(rest.iter().skip(2).cloned());
                } else {
                    args.extend(rest.iter().skip(1).cloned());
                }
            } else {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://persist-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "cowork" | "co-work" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "list".to_string());
            let mut args = vec!["cowork".to_string(), format!("--op={sub}")];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://persist-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "web" | "web-tooling" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let (script_rel, mut args) = match sub.as_str() {
                "auth" | "auth-presence" => (
                    "core://action-envelope-kernel",
                    vec!["web-auth-presence".to_string()],
                ),
                "query" | "query-shape" => (
                    "core://action-envelope-kernel",
                    vec!["web-query-shape".to_string()],
                ),
                "freshness" => (
                    "core://request-envelope-kernel",
                    vec!["normalize-web-freshness".to_string()],
                ),
                "date" => (
                    "core://request-envelope-kernel",
                    vec!["normalize-web-date".to_string()],
                ),
                _ => (
                    "core://upgrade-lane-kernel",
                    vec!["web-tooling-status".to_string()],
                ),
            };
            if !rest.is_empty() {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: script_rel.to_string(),
                args,
                forward_stdin: false,
            })
        }
        "app" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized_sub = match sub.as_str() {
                "run"
                | "status"
                | "history"
                | "replay"
                | "switch-provider"
                | "build"
                | "ingress"
                | "template-governance" => sub.as_str(),
                _ => "run",
            };
            let app_name = if sub == normalized_sub {
                rest.get(1)
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty() && !v.starts_with("--"))
                    .unwrap_or("chat-starter")
                    .to_string()
            } else {
                sub.clone()
            };
            let app = app_name.replace('_', "-").to_ascii_lowercase();
            let mut args = vec![normalized_sub.to_string(), format!("--app={}", app.trim())];
            let mut plain = Vec::<String>::new();
            let start_idx = if sub == normalized_sub {
                2usize
            } else {
                1usize
            };
            for token in rest.iter().skip(start_idx) {
                if token.starts_with("--") {
                    args.push(token.clone());
                } else {
                    plain.push(token.clone());
                }
            }
            if !plain.is_empty() {
                let joined = plain.join(" ");
                if app == "code-engineer" && normalized_sub == "build" {
                    args.push(format!("--goal={joined}"));
                } else if app == "code-engineer" {
                    args.push(format!("--prompt={joined}"));
                } else {
                    args.push(format!("--message={joined}"));
                }
            }
            Some(Route {
                script_rel: "core://app-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "chat-starter" | "chat_starter" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "run".to_string());
            let action = if matches!(sub.as_str(), "run" | "history" | "replay" | "status") {
                sub
            } else {
                "run".to_string()
            };
            let mut args = vec![action.clone(), "--app=chat-starter".to_string()];
            let start_idx = if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case(action.as_str()))
                .unwrap_or(false)
            {
                1usize
            } else {
                0usize
            };
            let mut plain = Vec::<String>::new();
            for token in rest.iter().skip(start_idx) {
                if token.starts_with("--") {
                    args.push(token.clone());
                } else {
                    plain.push(token.clone());
                }
            }
            if action == "run" && !plain.is_empty() {
                args.push(format!("--message={}", plain.join(" ")));
            } else if !plain.is_empty() {
                args.extend(plain);
            }
            Some(Route {
                script_rel: "core://app-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "chat-ui" | "chat_ui" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "run".to_string());
            let action = if matches!(
                sub.as_str(),
                "run" | "history" | "replay" | "status" | "switch-provider"
            ) {
                sub
            } else {
                "run".to_string()
            };
            let mut args = vec![action.clone(), "--app=chat-ui".to_string()];
            let start_idx = if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case(action.as_str()))
                .unwrap_or(false)
            {
                1usize
            } else {
                0usize
            };
            let mut plain = Vec::<String>::new();
            for token in rest.iter().skip(start_idx) {
                if token.starts_with("--") {
                    args.push(token.clone());
                } else {
                    plain.push(token.clone());
                }
            }
            if action == "run" && !plain.is_empty() {
                args.push(format!("--message={}", plain.join(" ")));
            } else if !plain.is_empty() {
                args.extend(plain);
            }
            Some(Route {
                script_rel: "core://app-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "code-engineer" | "code_engineer" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "run".to_string());
            let action = if matches!(
                sub.as_str(),
                "run" | "build" | "status" | "ingress" | "template-governance"
            ) {
                sub
            } else {
                "run".to_string()
            };
            let mut args = vec![action.clone(), "--app=code-engineer".to_string()];
            let mut plain = Vec::<String>::new();
            let start_idx = if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case(action.as_str()))
                .unwrap_or(false)
            {
                1usize
            } else {
                0usize
            };
            for token in rest.iter().skip(start_idx) {
                if token.starts_with("--") {
                    args.push(token.clone());
                } else {
                    plain.push(token.clone());
                }
            }
            if !plain.is_empty() {
                if action == "build" {
                    args.push(format!("--goal={}", plain.join(" ")));
                } else {
                    args.push(format!("--prompt={}", plain.join(" ")));
                }
            }
            Some(Route {
                script_rel: "core://app-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "build" => {
            let mut args = vec!["build".to_string(), "--app=code-engineer".to_string()];
            if !rest.is_empty() {
                let mut plain = Vec::<String>::new();
                for token in rest {
                    if token.starts_with("--") {
                        args.push(token.clone());
                    } else {
                        plain.push(token.clone());
                    }
                }
                if !plain.is_empty() {
                    args.push(format!("--goal={}", plain.join(" ")));
                }
            }
            Some(Route {
                script_rel: "core://app-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "snowball" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let first_is_flag = rest.first().map(|v| v.starts_with("--")).unwrap_or(false);
            let mut args = match sub.as_str() {
                "start" => vec!["start".to_string()],
                "melt" | "refine" | "melt-refine" | "regress" => {
                    vec!["melt-refine".to_string()]
                }
                "compact" => vec!["compact".to_string()],
                "backlog" | "backlog-pack" => vec!["backlog-pack".to_string()],
                "control" => vec!["control".to_string()],
                "status" => vec!["status".to_string()],
                _ => vec!["status".to_string()],
            };
            if !rest.is_empty() {
                if first_is_flag {
                    args.extend(rest.iter().cloned());
                } else {
                    args.extend(rest.iter().skip(1).cloned());
                }
            }
            Some(Route {
                script_rel: "core://snowball-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "schedule" => {
            let mut args = vec!["schedule".to_string()];
            args.extend(rest.iter().cloned());
            Some(Route {
                script_rel: "core://persist-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "mobile" => {
            let first = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let args = if first == "daemon" || first == "mobile-daemon" {
                let op = rest
                    .get(1)
                    .map(|v| v.trim().to_ascii_lowercase())
                    .filter(|v| !v.starts_with("--"))
                    .unwrap_or_else(|| "status".to_string());
                let mut routed = vec!["mobile-daemon".to_string(), format!("--op={op}")];
                let start = if rest
                    .get(1)
                    .map(|v| !v.trim().starts_with("--"))
                    .unwrap_or(false)
                {
                    2usize
                } else {
                    1usize
                };
                routed.extend(rest.iter().skip(start).cloned());
                routed
            } else {
                let mut routed = vec!["mobile-cockpit".to_string()];
                routed.extend(rest.iter().cloned());
                routed
            };
            Some(Route {
                script_rel: "core://persist-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "eye" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let mut args = match sub.as_str() {
                "enable" => vec!["eye-bind".to_string(), "--op=enable".to_string()],
                "status" => vec!["eye-bind".to_string(), "--op=status".to_string()],
                _ => vec!["eye-bind".to_string(), format!("--op={sub}")],
            };
            if let Some(source) = rest.get(1) {
                if !source.starts_with("--") {
                    args.push(format!("--source={}", source.trim()));
                    args.extend(rest.iter().skip(2).cloned());
                } else {
                    args.extend(rest.iter().skip(1).cloned());
                }
            } else {
                args.push("--source=wifi".to_string());
            }
            Some(Route {
                script_rel: "core://substrate-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "orchestrate"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("agency"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["orchestrate-agency".to_string()];
            if let Some(team) = rest.get(1) {
                if !team.starts_with("--") {
                    args.push(format!("--team={}", team.trim()));
                    args.extend(rest.iter().skip(2).cloned());
                } else {
                    args.extend(rest.iter().skip(1).cloned());
                }
            } else {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://company-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "shadow"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("browser"))
                .unwrap_or(false)
                || rest
                    .first()
                    .map(|v| v.trim().eq_ignore_ascii_case("--browser"))
                    .unwrap_or(false) =>
        {
            let mut args = vec![
                "session-start".to_string(),
                "--shadow=default-shadow".to_string(),
            ];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://vbrowser-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "shadow"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("delegate"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["delegate".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://hermes-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "shadow"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("continuity"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["continuity".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://hermes-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "shadow"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("create"))
                .unwrap_or(false)
                && rest.iter().any(|v| v.trim().starts_with("--template=")) =>
        {
            let mut args = vec!["create-shadow".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://agency-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "shadow"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("discover"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["discover".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://hermes-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "top" | "infring-top" => {
            let mut args = vec!["cockpit".to_string()];
            args.extend(rest.iter().cloned());
            Some(Route {
                script_rel: "core://hermes-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "status"
            if rest
                .iter()
                .any(|arg| arg.trim() == "dashboard" || arg.trim() == "--dashboard") =>
        {
            let mut args = vec!["cockpit".to_string()];
            args.extend(
                rest.iter()
                    .filter(|arg| arg.trim() != "dashboard" && arg.trim() != "--dashboard")
                    .cloned(),
            );
            Some(Route {
                script_rel: "core://hermes-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        _ => None,
    }
}
