pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("ternary_belief_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    let result = match command {
        "evaluate" => evaluate(root, &payload),
        "merge" => Ok(merge(&payload)),
        "serialize" => Ok(serialize(&payload)),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err("ternary_belief_kernel_unknown_command".to_string()),
    };
    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt("ternary_belief_kernel", payload));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("ternary_belief_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_returns_positive_belief() {
        let root = Path::new(".");
        let payload = json!({
            "signals": [
                {"source":"policy","trit":"ok","weight":1.0,"confidence":0.9},
                {"source":"health","trit":1,"weight":1.2,"confidence":0.95},
                {"source":"risk","trit":"unknown","weight":0.7,"confidence":0.8}
            ],
            "opts": {
                "label": "system_health",
                "source_trust": {"policy": 1.2, "health": 1.1},
                "force_neutral_on_insufficient_evidence": true
            }
        });
        let out = evaluate(root, payload.as_object().unwrap()).unwrap();
        assert_eq!(
            out["schema_id"],
            Value::String("ternary_belief".to_string())
        );
        assert_eq!(out["trit_label"], Value::String("ok".to_string()));
        assert_eq!(out["evidence_count"], Value::from(3));
    }

    #[test]
    fn merge_and_serialize_preserve_trit_vector() {
        let merged = merge(payload_obj(&json!({
            "parent": {"trit": 1, "score": 0.8, "confidence": 0.9},
            "child": {"trit": 1, "score": 0.6, "confidence": 0.7},
            "opts": {"mode": "cautious", "parent_weight": 1.0, "child_weight": 1.0}
        })));
        assert_eq!(merged["trit_label"], Value::String("ok".to_string()));
        let serialized = serialize(payload_obj(&json!({"belief": merged})));
        assert_eq!(
            serialized["vector"]["digits"],
            Value::String("+00".to_string())
        );
    }
}
