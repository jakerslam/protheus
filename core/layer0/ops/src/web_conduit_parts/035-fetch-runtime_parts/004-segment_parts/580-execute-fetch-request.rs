fn execute_fetch_request(root: &Path, request: &Value) -> Value {
    include!("004-segment_parts/580-execute-fetch-request-parts/010-part.rs");
    include!("004-segment_parts/580-execute-fetch-request-parts/020-tool-surface-ready.rs");
    include!("004-segment_parts/580-execute-fetch-request-parts/030-replay-enabled.rs");
    include!("004-segment_parts/580-execute-fetch-request-parts/040-tool-execution-allowed.rs");
    include!("004-segment_parts/580-execute-fetch-request-parts/050-attempt-replay-blocked.rs");
    include!("004-segment_parts/580-execute-fetch-request-parts/060-let.rs");
}
