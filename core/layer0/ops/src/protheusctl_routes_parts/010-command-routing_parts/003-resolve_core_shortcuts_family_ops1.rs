fn resolve_core_shortcuts_family_ops1(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "verity" => {
            let first = rest.first().map(|value| value.trim().to_ascii_lowercase());
            let (subcommand, passthrough_start_idx) = match first.as_deref() {
                Some("status") => ("status".to_string(), 1usize),
                Some("drift-status" | "drift") => ("drift-status".to_string(), 1usize),
                Some("vector-check" | "vector") => ("vector-check".to_string(), 1usize),
                Some("record-event" | "record") => ("record-event".to_string(), 1usize),
                Some("refine-event" | "refinement-event") => ("refine-event".to_string(), 1usize),
                _ => ("status".to_string(), 0usize),
            };
            Some(Route {
                script_rel: "core://verity-plane".to_string(),
                args: std::iter::once(subcommand)
                    .chain(rest.iter().skip(passthrough_start_idx).cloned())
                    .collect(),
                forward_stdin: false,
            })
        }
        "rag" => Some(Route {
            script_rel: "core://rag".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "swarm" => Some(Route {
            script_rel: "core://swarm-runtime".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
        "task" | "tasks" => {
            let mut args = vec!["task".to_string()];
            if rest.is_empty() {
                args.push("list".to_string());
            } else {
                args.extend(rest.iter().cloned());
            }
            Some(Route {
                script_rel: "core://workspace-gateway-runtime".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "web" | "browse" => {
            let mut args = Vec::<String>::new();
            if rest.is_empty() {
                args.push("status".to_string());
            } else {
                args.extend(rest.iter().cloned());
            }
            Some(Route {
                script_rel: "core://web-conduit".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "batch" | "batch-query" | "batch_query" => {
            let mut args = Vec::<String>::new();
            if rest.is_empty() {
                args.push("status".to_string());
            } else {
                args.extend(rest.iter().cloned());
            }
            Some(Route {
                script_rel: "core://batch-query".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "stack" | "context-stacks" => {
            let mut args = Vec::<String>::new();
            if rest.is_empty() {
                args.push("list".to_string());
            } else {
                args.extend(rest.iter().cloned());
            }
            Some(Route {
                script_rel: "core://context-stacks".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "workspace-search" | "workspace-files" => {
            let mut args = Vec::<String>::new();
            if rest.is_empty() {
                args.push("list".to_string());
            } else {
                args.extend(rest.iter().cloned());
            }
            Some(Route {
                script_rel: "core://workspace-file-search".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "doctor" | "verify-install" => {
            let mut args = vec![cmd.to_string()];
            args.extend(rest.iter().cloned());
            Some(Route {
                script_rel: "core://install-doctor".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "verify" => {
            let verify_target = rest
                .first()
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "runtime-proof".to_string());
            if verify_target == "runtime-proof" {
                let mut args = rest.iter().skip(1).cloned().collect::<Vec<_>>();
                if !contains_help_flag(&args)
                    && !args
                        .iter()
                        .any(|token| token.trim().starts_with("--strict"))
                {
                    args.push("--strict=1".to_string());
                }
                Some(Route {
                    script_rel: "tests/tooling/scripts/ci/runtime_proof_verify.ts".to_string(),
                    args,
                    forward_stdin: false,
                })
            } else if verify_target == "layer2-parity" || verify_target == "layer2_parity" {
                let mut args = rest.iter().skip(1).cloned().collect::<Vec<_>>();
                if !contains_help_flag(&args)
                    && !args
                        .iter()
                        .any(|token| token.trim().starts_with("--strict"))
                {
                    args.push("--strict=1".to_string());
                }
                Some(Route {
                    script_rel: "tests/tooling/scripts/ci/layer2_lane_parity_guard.ts".to_string(),
                    args,
                    forward_stdin: false,
                })
            } else if verify_target == "trusted-core" || verify_target == "trusted_core" {
                let mut args = rest.iter().skip(1).cloned().collect::<Vec<_>>();
                if !contains_help_flag(&args)
                    && !args
                        .iter()
                        .any(|token| token.trim().starts_with("--strict"))
                {
                    args.push("--strict=1".to_string());
                }
                Some(Route {
                    script_rel: "tests/tooling/scripts/ci/runtime_trusted_core_report.ts"
                        .to_string(),
                    args,
                    forward_stdin: false,
                })
            } else if verify_target == "release-proof-pack"
                || verify_target == "release_proof_pack"
                || verify_target == "proof-pack"
                || verify_target == "proof_pack"
            {
                let mut args = rest.iter().skip(1).cloned().collect::<Vec<_>>();
                if !contains_help_flag(&args)
                    && !args
                        .iter()
                        .any(|token| token.trim().starts_with("--strict"))
                {
                    args.push("--strict=1".to_string());
                }
                Some(Route {
                    script_rel: "tests/tooling/scripts/ci/release_proof_pack_assemble.ts"
                        .to_string(),
                    args,
                    forward_stdin: false,
                })
            } else if verify_target == "public-benchmark" || verify_target == "public_benchmark" {
                let mut args = rest.iter().skip(1).cloned().collect::<Vec<_>>();
                if !contains_help_flag(&args)
                    && !args
                        .iter()
                        .any(|token| token.trim().starts_with("--strict"))
                {
                    args.push("--strict=1".to_string());
                }
                Some(Route {
                    script_rel: "benchmarks/public_harness/run_public_harness.ts".to_string(),
                    args,
                    forward_stdin: false,
                })
            } else {
                let mut args = vec!["verify-install".to_string()];
                args.extend(rest.iter().cloned());
                Some(Route {
                    script_rel: "core://install-doctor".to_string(),
                    args,
                    forward_stdin: false,
                })
            }
        }
        "inspect" => {
            let inspect_target = rest
                .first()
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "boundedness".to_string());
            if inspect_target == "boundedness" {
                let mut args = rest.iter().skip(1).cloned().collect::<Vec<_>>();
                if !contains_help_flag(&args)
                    && !args
                        .iter()
                        .any(|token| token.trim().starts_with("--strict"))
                {
                    args.push("--strict=1".to_string());
                }
                Some(Route {
                    script_rel: "tests/tooling/scripts/ci/runtime_boundedness_inspect.ts"
                        .to_string(),
                    args,
                    forward_stdin: false,
                })
            } else {
                Some(Route {
                    script_rel: "core://unknown-command".to_string(),
                    args: std::iter::once("inspect".to_string())
                        .chain(rest.iter().cloned())
                        .collect(),
                    forward_stdin: false,
                })
            }
        }
        "replay" => {
            let replay_target = rest
                .first()
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "layer2".to_string());
            if replay_target == "layer2" || replay_target == "l2" || replay_target == "receipts" {
                let mut args = rest.iter().skip(1).cloned().collect::<Vec<_>>();
                if !contains_help_flag(&args)
                    && !args
                        .iter()
                        .any(|token| token.trim().starts_with("--strict"))
                {
                    args.push("--strict=1".to_string());
                }
                Some(Route {
                    script_rel: "tests/tooling/scripts/ci/layer2_receipt_replay.ts".to_string(),
                    args,
                    forward_stdin: false,
                })
            } else {
                Some(Route {
                    script_rel: "core://unknown-command".to_string(),
                    args: std::iter::once("replay".to_string())
                        .chain(rest.iter().cloned())
                        .collect(),
                    forward_stdin: false,
                })
            }
        }
        "dream" | "compact" | "proactive_daemon" | "speculate" => {
            let mut args = vec![cmd.to_string()];
            args.extend(rest.iter().cloned());
            Some(Route {
                script_rel: "core://autonomy-controller".to_string(),
                args,
                forward_stdin: false,
            })
        }
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
        "business" => Some(Route {
            script_rel: "core://business-plane".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            },
            forward_stdin: false,
        }),
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
    }
}
