use conduit_security::gateway_boundary::{
    run_gateway_boundary_guard, DEFAULT_GATEWAY_BOUNDARY_POLICY_PATH,
    DEFAULT_GATEWAY_BOUNDARY_REPORT_PATH,
};

fn main() {
    let mut policy_path = DEFAULT_GATEWAY_BOUNDARY_POLICY_PATH.to_string();
    let mut out_json = DEFAULT_GATEWAY_BOUNDARY_REPORT_PATH.to_string();
    let mut strict = true;
    for arg in std::env::args().skip(1) {
        if let Some(value) = arg.strip_prefix("--policy=") {
            policy_path = value.to_string();
        } else if let Some(value) = arg.strip_prefix("--out-json=") {
            out_json = value.to_string();
        } else if let Some(value) = arg.strip_prefix("--strict=") {
            strict = value != "0" && value != "false";
        }
    }
    match run_gateway_boundary_guard(policy_path.as_str(), out_json.as_str(), strict) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
            );
            if strict && !report.ok {
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
