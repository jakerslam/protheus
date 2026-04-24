use std::env;
use std::path::Path;

use infring_ops_core_v1::operator_critical_paths::{
    run_operator_critical_guard, DEFAULT_NODE_BURNDOWN_PLAN_PATH,
    DEFAULT_OPERATOR_CRITICAL_MARKDOWN_PATH, DEFAULT_OPERATOR_CRITICAL_REPORT_PATH,
};

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .find_map(|arg| arg.strip_prefix(&format!("--{key}=")).map(str::to_string))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let strict = arg_value(&args, "strict").unwrap_or_else(|| "1".to_string()) != "0";
    let mode = arg_value(&args, "mode").unwrap_or_else(|| "all".to_string());
    let plan = arg_value(&args, "plan").unwrap_or_else(|| DEFAULT_NODE_BURNDOWN_PLAN_PATH.to_string());
    let out_json = arg_value(&args, "out-json").unwrap_or_else(|| DEFAULT_OPERATOR_CRITICAL_REPORT_PATH.to_string());
    let out_markdown =
        arg_value(&args, "out-markdown").unwrap_or_else(|| DEFAULT_OPERATOR_CRITICAL_MARKDOWN_PATH.to_string());
    match run_operator_critical_guard(
        Path::new(&plan),
        Path::new(&out_json),
        Path::new(&out_markdown),
        &mode,
        strict,
    ) {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
