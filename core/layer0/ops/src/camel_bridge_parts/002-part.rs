
fn emit_error(
    _state: &mut Value,
    _state_path: &Path,
    _history_path: &Path,
    kind: &str,
    err: &str,
) -> i32 {
    print_json_line(&cli_error(kind, err));
    1
}
