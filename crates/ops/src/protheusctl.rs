use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::clean;

#[derive(Debug, Clone)]
pub struct Route {
    pub script_rel: String,
    pub args: Vec<String>,
    pub forward_stdin: bool,
}

#[derive(Debug, Clone)]
pub struct DispatchSecurity {
    pub ok: bool,
    pub reason: String,
}

fn bool_env(name: &str, fallback: bool) -> bool {
    match env::var(name) {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        Err(_) => fallback,
    }
}

fn node_bin() -> String {
    env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string())
}

fn parse_json(raw: &str) -> Option<Value> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return Some(v);
    }
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    for line in lines.iter().rev() {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            return Some(v);
        }
    }
    None
}

fn security_request(root: &Path, script_rel: &str, args: &[String]) -> Value {
    let digest_seed = serde_json::to_string(&json!({
        "script": script_rel,
        "args": args
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let mut hasher = Sha256::new();
    hasher.update(digest_seed.as_bytes());
    let digest = hex::encode(hasher.finalize());
    let now_ms = chrono::Utc::now().timestamp_millis();

    json!({
        "operation_id": clean(format!("protheusctl_dispatch_{}_{}", now_ms, &digest[..10]), 160),
        "subsystem": "ops",
        "action": "cli_dispatch",
        "actor": "systems/ops/protheusctl",
        "risk_class": if bool_env("PROTHEUS_CTL_SECURITY_HIGH_RISK", false) { "high" } else { "normal" },
        "payload_digest": format!("sha256:{digest}"),
        "tags": ["protheusctl", "dispatch", "foundation_lock"],
        "covenant_violation": bool_env("PROTHEUS_CTL_SECURITY_COVENANT_VIOLATION", false),
        "tamper_signal": bool_env("PROTHEUS_CTL_SECURITY_TAMPER_SIGNAL", false),
        "key_age_hours": env::var("PROTHEUS_CTL_SECURITY_KEY_AGE_HOURS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(1),
        "operator_quorum": env::var("PROTHEUS_CTL_SECURITY_OPERATOR_QUORUM").ok().and_then(|v| v.parse::<u8>().ok()).unwrap_or(2),
        "audit_receipt_nonce": clean(format!("nonce-{}-{}", &digest[..12], now_ms), 120),
        "zk_proof": clean(env::var("PROTHEUS_CTL_SECURITY_ZK_PROOF").unwrap_or_else(|_| "zk-protheusctl-dispatch".to_string()), 220),
        "ciphertext_digest": clean(format!("sha256:{}", &digest[..32]), 220),
        "state_root": clean(env::var("PROTHEUS_SECURITY_STATE_ROOT").unwrap_or_else(|_| root.join("state").to_string_lossy().to_string()), 500)
    })
}

pub fn evaluate_dispatch_security(
    root: &Path,
    script_rel: &str,
    args: &[String],
) -> DispatchSecurity {
    if bool_env("PROTHEUS_CTL_SECURITY_GATE_DISABLED", false) {
        return DispatchSecurity {
            ok: true,
            reason: "protheusctl_dispatch_gate_disabled".to_string(),
        };
    }

    let req = security_request(root, script_rel, args);
    if req
        .get("covenant_violation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || req
            .get("tamper_signal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:local_fail_closed_signal".to_string(),
        };
    }

    let request_json = serde_json::to_string(&req).unwrap_or_else(|_| "{}".to_string());
    let request_base64 = BASE64_STANDARD.encode(request_json.as_bytes());

    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg("crates/security/Cargo.toml")
        .arg("--bin")
        .arg("security_core")
        .arg("--")
        .arg("check")
        .arg(format!("--request-base64={request_base64}"))
        .current_dir(root)
        .output();

    let Ok(out) = output else {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:spawn_failed".to_string(),
        };
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let msg = if stderr.trim().is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        return DispatchSecurity {
            ok: false,
            reason: format!("security_gate_blocked:{}", clean(msg, 220)),
        };
    }

    let payload = parse_json(&String::from_utf8_lossy(&out.stdout));
    let Some(payload) = payload else {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:invalid_security_payload".to_string(),
        };
    };

    let decision = payload.get("decision").cloned().unwrap_or(Value::Null);
    let ok = decision.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let fail_closed = decision
        .get("fail_closed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if !ok || fail_closed {
        let reason = decision
            .get("reasons")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_str)
            .unwrap_or("dispatch_security_gate_blocked")
            .to_string();
        return DispatchSecurity {
            ok: false,
            reason: format!("security_gate_blocked:{}", clean(reason, 220)),
        };
    }

    DispatchSecurity {
        ok: true,
        reason: "ok".to_string(),
    }
}

fn run_node_script(root: &Path, script_rel: &str, args: &[String], forward_stdin: bool) -> i32 {
    let script_abs = root.join(script_rel);
    let mut cmd = Command::new(node_bin());
    cmd.arg(script_abs)
        .args(args)
        .current_dir(root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if forward_stdin {
        cmd.stdin(Stdio::inherit());
    } else {
        cmd.stdin(Stdio::null());
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "protheusctl_dispatch",
                    "error": clean(format!("spawn_failed:{err}"), 220)
                })
            );
            1
        }
    }
}

fn route_edge(rest: &[String]) -> Route {
    let sub = rest
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match sub.as_str() {
        "lifecycle" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "systems/edge/mobile_lifecycle_resilience.js".to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "swarm" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "systems/spawn/mobile_edge_swarm_bridge.js".to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "wrapper" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "systems/ops/mobile_wrapper_distribution_pack.js".to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "benchmark" => {
            let action = rest
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "systems/ops/mobile_competitive_benchmark_matrix.js".to_string(),
                args: std::iter::once(action)
                    .chain(rest.iter().skip(2).cloned())
                    .collect(),
                forward_stdin: false,
            }
        }
        "top" => Route {
            script_rel: "systems/edge/mobile_ops_top.js".to_string(),
            args: std::iter::once("status".to_string())
                .chain(rest.iter().skip(1).cloned())
                .collect(),
            forward_stdin: false,
        },
        _ => Route {
            script_rel: "systems/edge/protheus_edge_runtime.js".to_string(),
            args: std::iter::once(sub)
                .chain(rest.iter().skip(1).cloned())
                .collect(),
            forward_stdin: false,
        },
    }
}

pub fn usage() {
    println!("Usage: protheusctl <command> [flags]");
    println!("Examples:");
    println!("  protheus status");
    println!("  protheus health");
    println!("  protheusctl job-submit --kind=reconcile");
    println!("  protheusctl edge start --owner=jay --profile=mobile_seed");
    println!("  protheusctl rust run|report|status");
    println!("  protheusctl lens <persona> <query>");
    println!("  protheusctl orchestrate meeting \"topic\"");
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .cloned()
        .unwrap_or_else(|| "status".to_string());
    let rest = argv.iter().skip(1).cloned().collect::<Vec<_>>();

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let route = match cmd.as_str() {
        "status" => Route {
            script_rel: "systems/ops/protheus_control_plane.js".to_string(),
            args: std::iter::once("status".to_string()).chain(rest).collect(),
            forward_stdin: false,
        },
        "skills" if rest.first().map(String::as_str) == Some("discover") => Route {
            script_rel: "systems/ops/protheusctl_skills_discover.js".to_string(),
            args: rest.into_iter().skip(1).collect(),
            forward_stdin: false,
        },
        "edge" => route_edge(&rest),
        "host" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            Route {
                script_rel: "systems/ops/host_adaptation_operator_surface.js".to_string(),
                args: std::iter::once(sub)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "socket" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let args = match sub.as_str() {
                "list" => std::iter::once("lifecycle".to_string())
                    .chain(std::iter::once("list".to_string()))
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                "install" | "update" | "test" => std::iter::once("lifecycle".to_string())
                    .chain(std::iter::once(sub))
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                "admission" | "discover" | "activate" | "status" => std::iter::once(sub)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                _ => std::iter::once("status".to_string()).chain(rest).collect(),
            };
            Route {
                script_rel: "systems/ops/platform_socket_runtime.js".to_string(),
                args,
                forward_stdin: false,
            }
        }
        "mine" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "dashboard".to_string());
            Route {
                script_rel: "systems/economy/donor_mining_dashboard.js".to_string(),
                args: std::iter::once(sub)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "migrate" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            let supported = ["run", "status", "rollback", "help", "--help", "-h"];
            let args =
                if sub.is_empty() || sub.starts_with("--") || !supported.contains(&sub.as_str()) {
                    std::iter::once("run".to_string()).chain(rest).collect()
                } else if matches!(sub.as_str(), "help" | "--help" | "-h") {
                    vec!["help".to_string()]
                } else {
                    std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect()
                };
            Route {
                script_rel: "systems/migration/core_migration_bridge.js".to_string(),
                args,
                forward_stdin: false,
            }
        }
        "import" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            let supported = ["run", "status", "help", "--help", "-h"];
            let args =
                if sub.is_empty() || sub.starts_with("--") || !supported.contains(&sub.as_str()) {
                    std::iter::once("run".to_string()).chain(rest).collect()
                } else if matches!(sub.as_str(), "help" | "--help" | "-h") {
                    vec!["help".to_string()]
                } else {
                    std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect()
                };
            Route {
                script_rel: "systems/migration/universal_importers.js".to_string(),
                args,
                forward_stdin: false,
            }
        }
        "wasi2" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = if sub == "run" { "run" } else { "status" };
            Route {
                script_rel: "systems/ops/wasi2_execution_completeness_gate.js".to_string(),
                args: std::iter::once(normalized.to_string())
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "settle" => {
            let mut sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            let has_revert = rest
                .iter()
                .any(|arg| matches!(arg.as_str(), "--revert" | "--revert=1" | "--mode=revert"));
            if has_revert {
                sub = "revert".to_string();
            }
            let supported = [
                "list",
                "run",
                "run-all",
                "status",
                "settle",
                "revert",
                "edit-core",
                "edit-module",
                "edit",
            ];
            let args =
                if sub.is_empty() || sub.starts_with("--") || !supported.contains(&sub.as_str()) {
                    std::iter::once("settle".to_string()).chain(rest).collect()
                } else {
                    std::iter::once(sub)
                        .chain(rest.into_iter().skip(1))
                        .collect()
                };
            Route {
                script_rel: "systems/ops/settlement_program.js".to_string(),
                args,
                forward_stdin: false,
            }
        }
        "edit-core" => Route {
            script_rel: "systems/ops/settlement_program.js".to_string(),
            args: std::iter::once("edit-core".to_string())
                .chain(rest)
                .collect(),
            forward_stdin: false,
        },
        "edit" => Route {
            script_rel: "systems/ops/settlement_program.js".to_string(),
            args: if rest.is_empty() {
                vec!["edit-module".to_string()]
            } else {
                std::iter::once("edit-module".to_string())
                    .chain(rest)
                    .collect()
            },
            forward_stdin: false,
        },
        "scale" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = if ["list", "run", "run-all", "status"].contains(&sub.as_str()) {
                sub
            } else {
                "status".to_string()
            };
            Route {
                script_rel: "systems/ops/scale_readiness_program.js".to_string(),
                args: std::iter::once(normalized)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "perception" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = if ["list", "run", "run-all", "status"].contains(&sub.as_str()) {
                sub
            } else {
                "status".to_string()
            };
            Route {
                script_rel: "systems/ops/perception_polish_program.js".to_string(),
                args: std::iter::once(normalized)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "fluxlattice" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = if ["list", "run", "run-all", "status"].contains(&sub.as_str()) {
                sub
            } else {
                "status".to_string()
            };
            Route {
                script_rel: "systems/ops/fluxlattice_program.js".to_string(),
                args: std::iter::once(normalized)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "lensmap" => Route {
            script_rel: "packages/lensmap/lensmap_cli.js".to_string(),
            args: rest,
            forward_stdin: false,
        },
        "lens" => Route {
            script_rel: "systems/personas/cli.js".to_string(),
            args: rest,
            forward_stdin: true,
        },
        "arbitrate" => Route {
            script_rel: "systems/personas/cli.js".to_string(),
            args: std::iter::once("arbitrate".to_string())
                .chain(rest)
                .collect(),
            forward_stdin: true,
        },
        "orchestrate" => Route {
            script_rel: "systems/personas/orchestration.js".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest
            },
            forward_stdin: true,
        },
        "persona" => Route {
            script_rel: "systems/personas/cli.js".to_string(),
            args: if rest.is_empty() {
                vec!["--help".to_string()]
            } else {
                rest
            },
            forward_stdin: true,
        },
        "toolkit" => Route {
            script_rel: "systems/ops/cognitive_toolkit_cli.js".to_string(),
            args: if rest.is_empty() {
                vec!["list".to_string()]
            } else {
                rest
            },
            forward_stdin: true,
        },
        "spine" => Route {
            script_rel: "systems/spine/spine_safe_launcher.js".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest
            },
            forward_stdin: false,
        },
        "hold" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = if ["admit", "rehydrate", "simulate", "status"].contains(&sub.as_str())
            {
                sub
            } else {
                "status".to_string()
            };
            Route {
                script_rel: "systems/autonomy/hold_remediation_engine.js".to_string(),
                args: std::iter::once(normalized)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "rust" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = if ["run", "report", "status"].contains(&sub.as_str()) {
                sub
            } else {
                "status".to_string()
            };
            Route {
                script_rel: "systems/ops/rust_authoritative_microkernel_acceleration.js"
                    .to_string(),
                args: std::iter::once(normalized)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "rust-hybrid" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = if ["list", "run", "run-all", "status"].contains(&sub.as_str()) {
                sub
            } else {
                "status".to_string()
            };
            Route {
                script_rel: "systems/ops/rust_hybrid_migration_program.js".to_string(),
                args: std::iter::once(normalized)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "suite" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let normalized = if ["list", "run", "run-all", "status"].contains(&sub.as_str()) {
                sub
            } else {
                "status".to_string()
            };
            Route {
                script_rel: "systems/ops/productized_suite_program.js".to_string(),
                args: std::iter::once(normalized)
                    .chain(rest.into_iter().skip(1))
                    .collect(),
                forward_stdin: false,
            }
        }
        "rsi" => Route {
            script_rel: "adaptive/rsi/rsi_bootstrap.js".to_string(),
            args: if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest
            },
            forward_stdin: false,
        },
        "contract-lane" if rest.first().map(String::as_str) == Some("status") => Route {
            script_rel: "adaptive/rsi/rsi_bootstrap.js".to_string(),
            args: std::iter::once("contract-lane-status".to_string())
                .chain(rest.into_iter().skip(1))
                .collect(),
            forward_stdin: false,
        },
        "approve" if rest.iter().any(|arg| arg == "--rsi") => Route {
            script_rel: "adaptive/rsi/rsi_bootstrap.js".to_string(),
            args: std::iter::once("approve".to_string())
                .chain(rest.into_iter().filter(|arg| arg != "--rsi"))
                .collect(),
            forward_stdin: false,
        },
        _ => Route {
            script_rel: "systems/ops/protheus_control_plane.js".to_string(),
            args: std::iter::once(cmd).chain(rest).collect(),
            forward_stdin: false,
        },
    };

    let gate = evaluate_dispatch_security(root, &route.script_rel, &route.args);
    if !gate.ok {
        eprintln!(
            "{}",
            json!({
                "ok": false,
                "type": "protheusctl_dispatch_security_gate",
                "error": gate.reason
            })
        );
        return 1;
    }

    run_node_script(root, &route.script_rel, &route.args, route.forward_stdin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn route_edge_swarm_maps_correctly() {
        let route = route_edge(&[
            "swarm".to_string(),
            "enroll".to_string(),
            "--owner=jay".to_string(),
        ]);
        assert_eq!(
            route.script_rel,
            "systems/spawn/mobile_edge_swarm_bridge.js"
        );
        assert_eq!(route.args.first().map(String::as_str), Some("enroll"));
    }

    #[test]
    fn local_fail_closed_signal_blocks_dispatch() {
        std::env::set_var("PROTHEUS_CTL_SECURITY_GATE_DISABLED", "0");
        std::env::set_var("PROTHEUS_CTL_SECURITY_COVENANT_VIOLATION", "1");
        let root = PathBuf::from(".");
        let verdict =
            evaluate_dispatch_security(&root, "systems/ops/protheus_control_plane.js", &[]);
        assert!(!verdict.ok);
        assert!(verdict.reason.contains("fail_closed"));
        std::env::remove_var("PROTHEUS_CTL_SECURITY_COVENANT_VIOLATION");
    }
}
