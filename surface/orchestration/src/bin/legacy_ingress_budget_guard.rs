use infring_orchestration_surface_v1::legacy_ingress_budget::{
    write_legacy_ingress_budget_artifacts, LEGACY_INGRESS_BUDGET_ARTIFACT_JSON,
    LEGACY_INGRESS_BUDGET_ARTIFACT_MARKDOWN,
};
use std::env;
use std::process;

fn read_flag(args: &[String], name: &str, default_value: &str) -> String {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(prefix.as_str()).map(str::to_string))
        .unwrap_or_else(|| default_value.to_string())
}

fn strict_enabled(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--strict" || arg == "--strict=1" || arg == "--strict=true")
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let strict = strict_enabled(&args);
    let json_path = read_flag(&args, "--out-json", LEGACY_INGRESS_BUDGET_ARTIFACT_JSON);
    let markdown_path = read_flag(
        &args,
        "--out-markdown",
        LEGACY_INGRESS_BUDGET_ARTIFACT_MARKDOWN,
    );
    let root = env::current_dir().unwrap_or_else(|err| {
        eprintln!("failed to resolve current directory: {err}");
        process::exit(2);
    });
    let report = write_legacy_ingress_budget_artifacts(&root, &json_path, &markdown_path)
        .unwrap_or_else(|err| {
            eprintln!("failed to write legacy ingress budget artifacts: {err}");
            process::exit(2);
        });
    println!(
        "[legacy ingress budget] ok={} rows={} failures={}",
        report.ok,
        report.rows.len(),
        report.failures.len()
    );
    if strict && !report.ok {
        process::exit(1);
    }
}
