fn resolve_core_shortcuts_family_ops1_group_2(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
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
        "dream" | "compact" | "proactive_daemon" | "speculate" | "kairos" => {
            let routed_cmd = if cmd == "kairos" {
                "proactive_daemon"
            } else {
                cmd
            };
            let mut args = vec![routed_cmd.to_string()];
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
        _ => None,
    }
}
