// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use std::collections::BTreeSet;

use serde_json::{json, Value};

#[derive(Clone, Copy)]
pub enum CommandTier {
    Tier1,
    Experimental,
}

impl CommandTier {
    fn as_str(self) -> &'static str {
        match self {
            CommandTier::Tier1 => "tier1",
            CommandTier::Experimental => "experimental",
        }
    }
}

#[derive(Clone, Copy)]
pub enum CommandHandlerKind {
    CoreDomain,
    RuntimeScript,
}

impl CommandHandlerKind {
    fn as_str(self) -> &'static str {
        match self {
            CommandHandlerKind::CoreDomain => "core_domain",
            CommandHandlerKind::RuntimeScript => "runtime_script",
        }
    }
}

#[derive(Clone, Copy)]
pub struct CommandItem {
    synopsis: &'static str,
    desc: &'static str,
    tier: CommandTier,
    handler: CommandHandlerKind,
    script_rel: &'static str,
    read_only: bool,
    unsafe_surface: bool,
}

#[derive(Clone, Copy)]
pub struct Tier1RouteContract {
    pub cmd: &'static str,
    pub rest: &'static [&'static str],
    pub expected_script: &'static str,
}

const COMMANDS: &[CommandItem] = &[
    CommandItem {
        synopsis: "help",
        desc: "Show CLI help and command list.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::RuntimeScript,
        script_rel: "client/runtime/systems/ops/protheus_command_list.ts",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "list",
        desc: "Show compact command list.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::RuntimeScript,
        script_rel: "client/runtime/systems/ops/protheus_command_list.ts",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "gateway [start|stop|restart|status] [--gateway-persist=1|0]",
        desc:
            "One-command runtime gateway (boots rich dashboard + persistent supervisor by default).",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://daemon-control",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "status",
        desc: "Show daemon/control-plane status.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://daemon-control",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "dashboard",
        desc: "Launch unified dashboard web UI (localhost).",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://daemon-control",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "doctor",
        desc: "Run install/runtime diagnostics.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://install-doctor",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "verify-install",
        desc: "Run strict install/runtime verification checks.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://install-doctor",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "stack <create|list|archive|tail-merge|tail-promote|render|batch-class|scheduler-check|status|policy>",
        desc: "Manage context stacks for cacheable memory grouping.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://context-stacks",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "workspace-search <search|list|status> [--workspace=<path>] [--q=<query>] [--type=file|folder]",
        desc: "Search workspace files/folders with ripgrep-backed fuzzy ranking.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://workspace-file-search",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "alpha-check",
        desc: "Run alpha readiness checks.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://alpha-readiness",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "session <status|register|resume|send|list>",
        desc: "Manage command-center sessions.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://command-center-session",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "orchestration",
        desc: "Rust-core orchestration invoke surface (coordinator/scratchpad/checkpoint).",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://orchestration",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "swarm-runtime",
        desc: "Core swarm runtime lanes.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "capability-profile",
        desc: "Show hardware-sensed capability shedding profile.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://capability-profile",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:spawn",
        desc: "Spawn a governed swarm session.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:send",
        desc: "Send inter-agent message between sessions.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:receive",
        desc: "Receive pending inter-agent messages.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:ack",
        desc: "Acknowledge inter-agent message delivery.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:handoff",
        desc: "Perform a governed inter-agent handoff with lineage and context receipts.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:context",
        desc: "Put/get governed swarm context variables.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:bootstrap",
        desc: "Fetch the authoritative direct-send/budget bootstrap contract for a session.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:state",
        desc: "Inspect session state/context/tool history.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:query",
        desc: "Query swarm service discovery/result registry.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:tick",
        desc: "Advance persistent swarm check-ins.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:tools:register",
        desc: "Register a JSON-schema tool manifest over the governed swarm bridge.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:tools:invoke",
        desc: "Invoke a governed swarm tool manifest.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:stream:emit",
        desc: "Emit delimited swarm stream chunks with receipt anchors.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:stream:render",
        desc: "Render delimited swarm stream chunks for an agent turn.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:turns:run",
        desc: "Run a governed multi-turn swarm execution with fail-closed recovery.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:turns:show",
        desc: "Inspect a governed multi-turn swarm run receipt.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:networks:create",
        desc: "Create a composable governed swarm agent network.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:networks:status",
        desc: "Inspect a governed swarm agent network receipt.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:demo",
        desc: "Run the optional thin swarm REPL/demo shell over the governed bridge.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: true,
    },
    CommandItem {
        synopsis: "version",
        desc: "Print runtime version and build info.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::RuntimeScript,
        script_rel: "client/runtime/systems/ops/protheus_version_cli.ts",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "update",
        desc: "Check for the latest release and print upgrade guidance.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::RuntimeScript,
        script_rel: "client/runtime/systems/ops/protheus_version_cli.ts",
        read_only: true,
        unsafe_surface: false,
    },
];

const TIER1_ROUTE_CONTRACTS: &[Tier1RouteContract] = &[
    Tier1RouteContract {
        cmd: "gateway",
        rest: &["status"],
        expected_script: "core://daemon-control",
    },
    Tier1RouteContract {
        cmd: "dashboard",
        rest: &["status"],
        expected_script: "core://daemon-control",
    },
    Tier1RouteContract {
        cmd: "doctor",
        rest: &[],
        expected_script: "core://install-doctor",
    },
    Tier1RouteContract {
        cmd: "verify-install",
        rest: &[],
        expected_script: "core://install-doctor",
    },
];

const TIER1_RUNTIME_ENTRYPOINTS: &[&str] = &[
    "client/runtime/systems/ops/protheusd.ts",
    "client/runtime/systems/ops/protheus_status_dashboard.ts",
    "client/runtime/systems/ops/protheus_unknown_guard.ts",
    "client/runtime/systems/ops/protheus_completion.ts",
    "client/runtime/systems/ops/protheus_repl.ts",
    "client/runtime/systems/ops/protheus_command_list.ts",
    "client/runtime/systems/ops/protheus_version_cli.ts",
];

pub fn command_registry() -> &'static [CommandItem] {
    COMMANDS
}

pub fn tier1_route_contracts() -> &'static [Tier1RouteContract] {
    TIER1_ROUTE_CONTRACTS
}

pub fn tier1_runtime_entrypoints() -> Vec<&'static str> {
    let mut out = TIER1_RUNTIME_ENTRYPOINTS.to_vec();
    out.sort_unstable();
    out
}

pub fn tier1_command_synopses() -> Vec<&'static str> {
    COMMANDS
        .iter()
        .filter(|row| matches!(row.tier, CommandTier::Tier1))
        .map(|row| row.synopsis)
        .collect::<Vec<_>>()
}

pub fn command_registry_integrity() -> Value {
    let mut seen = BTreeSet::<String>::new();
    let mut duplicates = Vec::<String>::new();
    let mut tier1 = 0usize;
    let mut experimental = 0usize;
    for row in COMMANDS {
        let key = row.synopsis.to_ascii_lowercase();
        if !seen.insert(key) {
            duplicates.push(row.synopsis.to_string());
        }
        match row.tier {
            CommandTier::Tier1 => tier1 += 1,
            CommandTier::Experimental => experimental += 1,
        }
    }
    duplicates.sort_unstable();
    json!({
        "ok": duplicates.is_empty(),
        "total": COMMANDS.len(),
        "tier1": tier1,
        "experimental": experimental,
        "duplicates": duplicates,
        "tier1_runtime_entrypoints": tier1_runtime_entrypoints(),
    })
}

fn parse_args(argv: &[String]) -> (String, bool) {
    let mut mode = "list".to_string();
    let mut json_out = false;
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim();
        if let Some(raw) = token.strip_prefix("--mode=") {
            let cleaned = raw.trim().to_ascii_lowercase();
            if !cleaned.is_empty() {
                mode = cleaned;
            }
        } else if token == "--mode" {
            if let Some(next) = argv.get(i + 1).map(|v| v.trim()).filter(|v| !v.is_empty()) {
                mode = next.to_ascii_lowercase();
                i += 1;
            }
        } else if token == "--json" || token == "--json=1" {
            json_out = true;
        } else if token == "help" || token == "--help" || token == "-h" {
            mode = "help".to_string();
        } else if token == "registry" {
            mode = "registry".to_string();
        }
        i += 1;
    }
    (mode, json_out)
}

fn commands_json() -> Value {
    Value::Array(
        COMMANDS
            .iter()
            .map(|row| {
                json!({
                    "synopsis": row.synopsis,
                    "desc": row.desc,
                    "tier": row.tier.as_str(),
                    "handler": row.handler.as_str(),
                    "script_rel": row.script_rel,
                    "read_only": row.read_only,
                    "unsafe_surface": row.unsafe_surface
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn print_list() {
    println!("InfRing command list (tier1 first):");
    for row in COMMANDS {
        println!("  - {:<12} {}", row.tier.as_str(), row.synopsis);
    }
}

fn print_help() {
    println!("Usage: infring <command> [flags]");
    println!();
    println!("Tier1 (stable product surface):");
    for row in COMMANDS {
        if matches!(row.tier, CommandTier::Tier1) {
            println!("  {:<45} {}", row.synopsis, row.desc);
        }
    }
    println!();
    println!("Experimental:");
    for row in COMMANDS {
        if matches!(row.tier, CommandTier::Experimental) {
            println!("  {:<45} {}", row.synopsis, row.desc);
        }
    }
}

fn print_registry_summary() {
    let summary = command_registry_integrity();
    println!(
        "Command registry: total={}, tier1={}, experimental={}",
        summary.get("total").and_then(Value::as_u64).unwrap_or(0),
        summary.get("tier1").and_then(Value::as_u64).unwrap_or(0),
        summary
            .get("experimental")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    let duplicates = summary
        .get("duplicates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if duplicates.is_empty() {
        println!("Registry integrity: ok");
    } else {
        println!("Registry integrity: duplicate command descriptors found");
    }
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let (mode, json_out) = parse_args(argv);
    if json_out {
        println!(
            "{}",
            json!({
                "ok": true,
                "type": "infring_command_list",
                "mode": mode,
                "commands": commands_json(),
                "registry_integrity": command_registry_integrity(),
                "route_contracts": tier1_route_contracts()
                    .iter()
                    .map(|row| {
                        json!({
                            "cmd": row.cmd,
                            "rest": row.rest,
                            "expected_script": row.expected_script
                        })
                    })
                    .collect::<Vec<_>>(),
            })
        );
        return 0;
    }

    match mode.as_str() {
        "help" => print_help(),
        "registry" => print_registry_summary(),
        _ => print_list(),
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_detects_mode_and_json() {
        let argv = vec!["--mode=help".to_string(), "--json".to_string()];
        let (mode, json_out) = parse_args(&argv);
        assert_eq!(mode, "help");
        assert!(json_out);
    }

    #[test]
    fn commands_json_has_expected_shape() {
        let out = commands_json();
        let rows = out.as_array().cloned().unwrap_or_default();
        assert!(!rows.is_empty());
        let first = rows
            .first()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(first.get("synopsis").and_then(Value::as_str), Some("help"));
        assert_eq!(first.get("tier").and_then(Value::as_str), Some("tier1"));
    }

    #[test]
    fn tier1_runtime_entrypoints_are_unique() {
        let entries = tier1_runtime_entrypoints();
        let set: BTreeSet<String> = entries.iter().map(|row| (*row).to_string()).collect();
        assert_eq!(entries.len(), set.len());
        assert!(entries
            .iter()
            .any(|row| *row == "client/runtime/systems/ops/protheus_command_list.ts"));
    }

    #[test]
    fn command_registry_integrity_reports_no_duplicates() {
        let summary = command_registry_integrity();
        assert_eq!(summary.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            summary
                .get("duplicates")
                .and_then(Value::as_array)
                .map(|rows| rows.is_empty()),
            Some(true)
        );
    }
}
