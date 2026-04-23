fn resolve_core_shortcuts_family_ops1_group_3(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "canyon" => Some(Route {
            script_rel: "core://canyon-plane".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "init" => {
            if contains_help_flag(rest) {
                return Some(Route {
                    script_rel: "core://canyon-plane".to_string(),
                    args: vec!["help".to_string()],
                    forward_stdin: false,
                });
            }
            let mut args = vec!["ecosystem".to_string(), "--op=init".to_string()];
            let pure_requested = parse_true_flag(rest, "pure");
            let tiny_max_requested =
                parse_true_flag(rest, "tiny-max") || parse_true_flag(rest, "tiny_max");
            if (pure_requested || tiny_max_requested) && !has_prefix_flag(rest, "workspace-mode") {
                args.push("--workspace-mode=pure".to_string());
            }
            if tiny_max_requested && !has_prefix_flag(rest, "pure") {
                args.push("--pure=1".to_string());
            }
            if let Some(template) = rest.first() {
                if !template.starts_with("--") {
                    args.push(format!("--template={}", template.trim()));
                    args.extend(rest.iter().skip(1).cloned());
                } else {
                    args.extend(rest.iter().cloned());
                }
            }
            Some(Route {
                script_rel: "core://canyon-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "alpha-check" => Some(Route {
            script_rel: "core://alpha-readiness".to_string(),
            args: if rest.is_empty() {
                vec!["run".to_string()]
            } else if rest
                .first()
                .map(|value| value.trim().starts_with("--"))
                .unwrap_or(false)
            {
                std::iter::once("run".to_string())
                    .chain(rest.iter().cloned())
                    .collect::<Vec<_>>()
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "marketplace" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let op = match sub.as_str() {
                "publish" => "marketplace-publish",
                "install" => "marketplace-install",
                _ => "marketplace-status",
            };
            let mut args = vec!["ecosystem".to_string(), format!("--op={op}")];
            if !rest.is_empty() {
                args.extend(rest.iter().skip(1).cloned());
            }
            Some(Route {
                script_rel: "core://canyon-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "government" | "gov" => Some(Route {
            script_rel: "core://government-plane".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "finance" | "bank" => Some(Route {
            script_rel: "core://finance-plane".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "healthcare" | "hospital" => Some(Route {
            script_rel: "core://healthcare-plane".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "vertical" => Some(Route {
            script_rel: "core://vertical-plane".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "nexus" => Some(Route {
            script_rel: "core://nexus-plane".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "instinct" => Some(Route {
            script_rel: "core://instinct-bridge".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "phone" => Some(Route {
            script_rel: "core://phone-runtime-bridge".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "adaptive" | "adaptive-intelligence" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = match sub.as_str() {
                "shadow_train" => "shadow-train",
                "status" | "propose" | "shadow-train" | "prioritize" | "graduate" => sub.as_str(),
                _ => "status",
            };
            let mut args = vec![normalized.to_string()];
            if !rest.is_empty() {
                if normalized == sub {
                    args.extend(rest.iter().skip(1).cloned());
                } else {
                    args.extend(rest.iter().cloned());
                }
            }
            Some(Route {
                script_rel: "core://adaptive-intelligence".to_string(),
                args,
                forward_stdin: false,
            })
        }
        _ => resolve_core_shortcuts_family_ops2(cmd, rest),
        _ => None,
    }
}
