#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_nexus_message() {
        let parsed = parse_nexus_message("[AG1>COORD|swarm] ROUT Q=H BP=M").expect("parse");
        assert_eq!(parsed.from, "AG1");
        assert_eq!(parsed.to, "COORD");
        assert_eq!(parsed.module.as_deref(), Some("swarm"));
        assert_eq!(parsed.cmd, "ROUT");
        assert_eq!(parsed.kv.get("Q").map(String::as_str), Some("H"));
    }

    #[test]
    fn rejects_invalid_message_without_header() {
        let err = parse_nexus_message("AG1>COORD ROUT Q=H").expect_err("must fail");
        assert!(err.contains("missing_header_open"));
    }

    #[test]
    fn enforces_module_limit() {
        let argv = vec!["--modules=a,b,c,d".to_string()];
        let err = parse_modules(&argv).expect_err("must fail");
        assert!(err.contains("module_limit_exceeded"));
    }

    #[test]
    fn compress_round_trip_keeps_strict_format() {
        let modules = vec!["swarm".to_string()];
        let lexicon = active_lexicon(&modules).expect("lexicon");
        let reverse = reverse_lexicon(&lexicon);
        let (msg, fallback) = compress_text_to_message(
            "ag1",
            "coord",
            Some("swarm".to_string()),
            "ROUT",
            "backpressure queue_depth",
            &reverse,
        );
        assert!(!fallback);
        let line = format_nexus_message(&msg);
        let reparsed = parse_nexus_message(&line).expect("reparse");
        let decompressed = decompress_message(&reparsed, &lexicon);
        assert_eq!(
            decompressed
                .get("cmd")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "route"
        );
    }

    #[test]
    fn send_persists_receipt_and_burn_metrics() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let argv = vec![
            "send".to_string(),
            "--message=[AG1>COORD|swarm] ROUT Q=H BP=M".to_string(),
            "--modules=swarm".to_string(),
            "--raw-text=backpressure queue_depth".to_string(),
        ];
        let (_payload, code) = send_command(root, &argv);
        assert_eq!(code, 0);
        let latest = read_json(&latest_path(root)).expect("latest");
        assert!(
            latest
                .get("total_nexus_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        );
    }
}
