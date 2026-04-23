const ASSIMILATE_SCRIPT: &str = "client/runtime/systems/tools/assimilation_cli_bridge.ts";
const SETUP_WIZARD_SCRIPT: &str = "client/runtime/systems/ops/infring_setup_wizard.ts";
const DEMO_SCRIPT: &str = "client/runtime/systems/ops/infring_demo.js";
const EXAMPLES_SCRIPT: &str = "client/runtime/systems/ops/infring_examples.js";
const DIAGRAM_SCRIPT: &str = "client/runtime/systems/ops/infring_diagram.js";
const VERSION_SCRIPT_JS: &str = "client/runtime/systems/ops/infring_version_cli.js";
const COMPLETION_SCRIPT_JS: &str = "client/runtime/systems/ops/infring_completion.js";

fn dashboard_ui_compat_enabled(rest: &[String]) -> bool {
    if bool_env("INFRING_ENABLE_DASHBOARD_UI_ALIAS", false) {
        return true;
    }
    rest.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--compat-dashboard-ui"
                | "--compat-dashboard-ui=1"
                | "--dashboard-ui-compat"
                | "--dashboard-ui-compat=1"
        )
    })
}

fn strip_dashboard_ui_compat_flags(rest: Vec<String>) -> Vec<String> {
    rest.into_iter()
        .filter(|arg| {
            !matches!(
                arg.as_str(),
                "--compat-dashboard-ui"
                    | "--compat-dashboard-ui=1"
                    | "--dashboard-ui-compat"
                    | "--dashboard-ui-compat=1"
            )
        })
        .collect::<Vec<_>>()
}

fn resolve_assimilate_route(rest: &[String]) -> Route {
    let default_args = if rest.is_empty() {
        vec!["--help".to_string()]
    } else {
        rest.to_vec()
    };
    if rest.is_empty() {
        return Route {
            script_rel: ASSIMILATE_SCRIPT.to_string(),
            args: default_args,
            forward_stdin: false,
        };
    }

    let (target, mut core_passthrough, wrapper_flags) = split_assimilate_tokens(rest);
    let Some(target_value) = target else {
        return Route {
            script_rel: ASSIMILATE_SCRIPT.to_string(),
            args: default_args,
            forward_stdin: false,
        };
    };
    let mut core_rest = vec![target_value.clone()];
    core_rest.append(&mut core_passthrough);

    let Some(core_route) = resolve_core_shortcuts("assimilate", &core_rest) else {
        return Route {
            script_rel: ASSIMILATE_SCRIPT.to_string(),
            args: default_args,
            forward_stdin: false,
        };
    };

    let Some(core_domain) = core_route.script_rel.strip_prefix("core://") else {
        return core_route;
    };
    let encoded_core_args = serde_json::to_string(&core_route.args)
        .map(|raw| BASE64_STANDARD.encode(raw.as_bytes()))
        .unwrap_or_else(|_| BASE64_STANDARD.encode(b"[]"));
    let mut args = vec![
        format!("--target={}", target_value),
        format!("--core-domain={}", core_domain),
        format!("--core-args-base64={}", encoded_core_args),
    ];
    args.extend(wrapper_flags);
    Route {
        script_rel: ASSIMILATE_SCRIPT.to_string(),
        args,
        forward_stdin: false,
    }
}

pub fn usage() {
    println!("Usage: infring <command> [flags]");
    println!("Try:");
    println!("  infring gateway");
    println!("  infring dream");
    println!("  infring compact");
    println!("  infring proactive_daemon");
    println!("  infring kairos");
    println!("  infring speculate");
    println!("  infring dashboard");
    println!("  infring verify runtime-proof --profile=rich");
    println!("  infring verify layer2-parity");
    println!("  infring verify trusted-core");
    println!("  infring verify release-proof-pack --version=2026-04-16");
    println!("  infring inspect boundedness --profile=rich");
    println!("  infring replay layer2 --bundle=tests/tooling/fixtures/layer2_receipt_bundle_golden.json");
    println!("  infring task list");
    println!("  infring list");
    println!("  infring --help");
    println!("  infring setup");
}

fn route_script_exists(root: &Path, script_rel: &str) -> bool {
    if script_rel.starts_with("core://") {
        true
    } else {
        root.join(script_rel).exists()
    }
}

fn command_mode_reason(cmd: &str, route: &Route) -> Option<&'static str> {
    let first_arg = route
        .args
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    match cmd {
        "gateway" => {
            if route.script_rel == "core://daemon-control"
                && ![
                    "start",
                    "stop",
                    "restart",
                    "status",
                    "heal",
                    "attach",
                    "subscribe",
                    "tick",
                    "diagnostics",
                    "watchdog",
                ]
                .contains(&first_arg.as_str())
            {
                return Some("gateway_mode_invalid");
            }
        }
        "status" => {
            if route.script_rel == "core://daemon-control" && first_arg != "status" {
                return Some("status_mode_invalid");
            }
        }
        "dashboard" => {
            if route.script_rel == "core://daemon-control" && first_arg != "start" {
                return Some("dashboard_mode_invalid");
            }
        }
        "dream" | "compact" | "proactive_daemon" | "speculate" => {
            if route.script_rel == "core://autonomy-controller" && first_arg != cmd {
                return Some("autonomy_mode_invalid");
            }
        }
        _ => {}
    }
    None
}
