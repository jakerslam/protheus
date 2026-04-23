fn maybe_redirect_ts_wrapper_to_core_domain(script_rel: &str, args: &[String]) -> Option<(String, Vec<String>)> {
    match script_rel {
        "client/runtime/systems/ops/infring_control_plane.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let normalized = match sub.as_str() {
                "health" => "status".to_string(),
                "job-submit" | "audit" => "run".to_string(),
                _ => sub,
            };
            let mapped = std::iter::once(normalized)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("infring-control-plane".to_string(), mapped))
        }
        "client/runtime/systems/ops/infring_debug_diagnostics.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let mapped = if matches!(sub.as_str(), "status" | "health") {
                std::iter::once("status".to_string())
                    .chain(args.iter().skip(1).cloned())
                    .collect::<Vec<_>>()
            } else {
                std::iter::once("run".to_string())
                    .chain(args.iter().cloned())
                    .collect::<Vec<_>>()
            };
            Some(("infring-control-plane".to_string(), mapped))
        }
        "client/runtime/systems/ops/infring_status_dashboard.ts" => Some((
            "daemon-control".to_string(),
            normalize_status_dashboard_args(args),
        )),
        "client/runtime/systems/ops/infring_unknown_guard.ts" => Some((
            "unknown-command".to_string(),
            args.to_vec(),
        )),
        "client/runtime/systems/ops/backlog_github_sync.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let mapped = std::iter::once(sub)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("backlog-github-sync".to_string(), mapped))
        }
        "client/runtime/systems/ops/backlog_registry.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let normalized = if matches!(sub.as_str(), "metrics" | "triage") {
                "status".to_string()
            } else {
                sub
            };
            let mapped = std::iter::once(normalized)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("backlog-registry".to_string(), mapped))
        }
        "client/runtime/systems/ops/rust50_migration_program.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let mapped = std::iter::once(sub)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("rust50-migration-program".to_string(), mapped))
        }
        "client/runtime/systems/ops/rust_enterprise_productivity_program.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let mapped = std::iter::once(sub)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("rust-enterprise-productivity-program".to_string(), mapped))
        }
        "client/runtime/systems/ops/rust_hotpath_inventory.ts" => Some((
            "rust-hotpath-inventory-kernel".to_string(),
            normalized_rust_hotpath_inventory_args(args),
        )),
        "client/runtime/systems/ops/benchmark_autonomy_gate.ts" => Some((
            "benchmark-autonomy-gate".to_string(),
            if args.is_empty() {
                vec!["run".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/ops/generate_coverage_badge.ts" => Some((
            "coverage-badge-kernel".to_string(),
            normalized_coverage_badge_args(args),
        )),
        "client/runtime/systems/ops/local_runtime_partitioner.ts" => Some((
            "local-runtime-partitioner".to_string(),
            normalized_local_runtime_partitioner_args(args),
        )),
        "client/runtime/systems/ops/security_layer_inventory_gate.ts" => Some((
            "security-layer-inventory-gate-kernel".to_string(),
            if args.is_empty() {
                vec!["run".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/ops/top50_roi_sweep.ts" => Some((
            "top50-roi-sweep-kernel".to_string(),
            normalized_top50_roi_args(args),
        )),
        "client/runtime/systems/ops/readiness_bridge_pack.ts" => Some((
            "readiness-bridge-pack-kernel".to_string(),
            if args.is_empty() {
                vec!["run".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/ops/system_health_audit_runner.ts" => Some((
            "system-health-audit-runner-kernel".to_string(),
            if args.is_empty() {
                vec!["run".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/ops/release_semver_contract.ts" => Some((
            "release-semver-contract".to_string(),
            if args.is_empty() {
                vec!["status".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/continuity/resurrection_protocol.ts" => Some((
            "continuity-runtime".to_string(),
            std::iter::once("resurrection-protocol".to_string())
                .chain(normalize_resurrection_protocol_args(args))
                .collect::<Vec<_>>(),
        )),
        "client/runtime/systems/continuity/session_continuity_vault.ts" => Some((
            "continuity-runtime".to_string(),
            std::iter::once("session-continuity-vault".to_string())
                .chain(normalize_session_continuity_vault_args(args))
                .collect::<Vec<_>>(),
        )),
        "client/runtime/systems/continuity/sovereign_resurrection_substrate.ts" => Some((
            "runtime-systems".to_string(),
            std::iter::once(
                "--system-id=SYSTEMS-CONTINUITY-SOVEREIGN_RESURRECTION_SUBSTRATE".to_string(),
            )
            .chain(args.iter().cloned())
            .collect::<Vec<_>>(),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod cli_domain_wrapper_redirect_tests {
    use super::*;

    #[test]
    fn debug_wrapper_redirects_to_control_plane() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/infring_debug_diagnostics.ts",
            &["status".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "infring-control-plane");
        assert_eq!(args, vec!["status"]);
    }

    #[test]
    fn continuity_wrapper_redirects_with_surface_prefix() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/continuity/session_continuity_vault.ts",
            &["restore".to_string(), "--id=s1".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "continuity-runtime");
        assert_eq!(
            args,
            vec![
                "session-continuity-vault".to_string(),
                "get".to_string(),
                "--id=s1".to_string()
            ]
        );
    }

    #[test]
    fn rust_hotpath_wrapper_redirects_to_kernel_with_status_default() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/rust_hotpath_inventory.ts",
            &["--policy=foo.json".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "rust-hotpath-inventory-kernel");
        assert_eq!(args, vec!["status".to_string(), "--policy=foo.json".to_string()]);
    }

    #[test]
    fn top50_wrapper_redirects_with_run_default() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/top50_roi_sweep.ts",
            &["--max=25".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "top50-roi-sweep-kernel");
        assert_eq!(args, vec!["run".to_string(), "--max=25".to_string()]);
    }

    #[test]
    fn release_semver_wrapper_redirects_to_core_domain() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/release_semver_contract.ts",
            &["run".to_string(), "--write=1".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "release-semver-contract");
        assert_eq!(args, vec!["run".to_string(), "--write=1".to_string()]);
    }

    #[test]
    fn status_dashboard_wrapper_redirects_web_to_daemon_start() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/infring_status_dashboard.ts",
            &["--web".to_string(), "--dashboard-open=1".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "daemon-control");
        assert_eq!(
            args,
            vec!["start".to_string(), "--dashboard-open=1".to_string()]
        );
    }
}

