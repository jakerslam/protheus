
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy = load_policy(root, parsed.flags.get("policy"));
    let strict = bool_flag(parsed.flags.get("strict"), policy.strict_default);
    let pr_body_path = resolve_path(
        root,
        parsed.flags.get("pr-body-path").map(String::as_str),
        "local/state/ops/sdlc_change_control/pr_body.md",
    );
    let changed_paths_path = resolve_path(
        root,
        parsed.flags.get("changed-paths-path").map(String::as_str),
        "local/state/ops/sdlc_change_control/changed_paths.txt",
    );

    match cmd.as_str() {
        "run" => match run_cmd(root, &policy, strict, &pr_body_path, &changed_paths_path) {
            Ok((payload, code)) => {
                print_json_line(&payload);
                code
            }
            Err(err) => {
                print_json_line(&cli_error_receipt(argv, &format!("run_failed:{err}"), 1));
                1
            }
        },
        "status" => {
            print_json_line(&status_cmd(&policy));
            0
        }
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}
