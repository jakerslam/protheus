
pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "collector_runtime_kernel") {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "collector_runtime_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(value) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt(
                "collector_runtime_kernel",
                value,
            ));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "collector_runtime_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
#[path = "collector_runtime_kernel_tests.rs"]
mod collector_runtime_kernel_tests;

