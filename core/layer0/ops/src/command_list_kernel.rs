// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};

#[derive(Clone, Copy)]
struct CommandItem {
    name: &'static str,
    desc: &'static str,
}

const COMMANDS: &[CommandItem] = &[
    CommandItem {
        name: "help",
        desc: "Show CLI help and command list.",
    },
    CommandItem {
        name: "gateway [start|stop|restart|status] [--gateway-persist=1|0]",
        desc:
            "One-command runtime gateway (boots rich dashboard + persistent supervisor by default).",
    },
    CommandItem {
        name: "status",
        desc: "Show daemon/control-plane status.",
    },
    CommandItem {
        name: "dashboard",
        desc: "Launch unified dashboard web UI (localhost).",
    },
    CommandItem {
        name: "alpha-check",
        desc: "Run alpha readiness checks.",
    },
    CommandItem {
        name: "session <status|register|resume|send|list>",
        desc: "Manage command-center sessions.",
    },
    CommandItem {
        name: "orchestration",
        desc: "Rust-core orchestration invoke surface (coordinator/scratchpad/checkpoint).",
    },
    CommandItem {
        name: "swarm-runtime",
        desc: "Core swarm runtime lanes.",
    },
    CommandItem {
        name: "capability-profile",
        desc: "Show hardware-sensed capability shedding profile.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:spawn",
        desc: "Spawn a governed swarm session.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:send",
        desc: "Send inter-agent message between sessions.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:receive",
        desc: "Receive pending inter-agent messages.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:ack",
        desc: "Acknowledge inter-agent message delivery.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:handoff",
        desc: "Perform a governed inter-agent handoff with lineage and context receipts.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:context",
        desc: "Put/get governed swarm context variables.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:bootstrap",
        desc: "Fetch the authoritative direct-send/budget bootstrap contract for a session.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:state",
        desc: "Inspect session state/context/tool history.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:query",
        desc: "Query swarm service discovery/result registry.",
    },
    CommandItem {
        name: "autonomy:swarm:sessions:tick",
        desc: "Advance persistent swarm check-ins.",
    },
    CommandItem {
        name: "autonomy:swarm:tools:register",
        desc: "Register a JSON-schema tool manifest over the governed swarm bridge.",
    },
    CommandItem {
        name: "autonomy:swarm:tools:invoke",
        desc: "Invoke a governed swarm tool manifest.",
    },
    CommandItem {
        name: "autonomy:swarm:stream:emit",
        desc: "Emit delimited swarm stream chunks with receipt anchors.",
    },
    CommandItem {
        name: "autonomy:swarm:stream:render",
        desc: "Render delimited swarm stream chunks for an agent turn.",
    },
    CommandItem {
        name: "autonomy:swarm:turns:run",
        desc: "Run a governed multi-turn swarm execution with fail-closed recovery.",
    },
    CommandItem {
        name: "autonomy:swarm:turns:show",
        desc: "Inspect a governed multi-turn swarm run receipt.",
    },
    CommandItem {
        name: "autonomy:swarm:networks:create",
        desc: "Create a composable governed swarm agent network.",
    },
    CommandItem {
        name: "autonomy:swarm:networks:status",
        desc: "Inspect a governed swarm agent network receipt.",
    },
    CommandItem {
        name: "autonomy:swarm:demo",
        desc: "Run the optional thin swarm REPL/demo shell over the governed bridge.",
    },
    CommandItem {
        name: "version",
        desc: "Print runtime version and build info.",
    },
];

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
        }
        i += 1;
    }
    (mode, json_out)
}

fn commands_json() -> Value {
    Value::Array(
        COMMANDS
            .iter()
            .map(|row| json!({ "name": row.name, "desc": row.desc }))
            .collect::<Vec<_>>(),
    )
}

fn print_list() {
    println!("InfRing command list:");
    for row in COMMANDS {
        println!("  - {}", row.name);
    }
}

fn print_help() {
    println!("Usage: infring <command> [flags]");
    println!();
    println!("High-signal commands:");
    for row in COMMANDS {
        println!("  {:<45} {}", row.name, row.desc);
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
            })
        );
        return 0;
    }

    if mode == "help" {
        print_help();
    } else {
        print_list();
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
        assert_eq!(first.get("name").and_then(Value::as_str), Some("help"));
    }
}
