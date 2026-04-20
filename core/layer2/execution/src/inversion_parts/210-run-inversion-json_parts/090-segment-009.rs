            "mode": "extract_first_principle",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_extract_first_principle_failed:{e}"));
    }
    if mode == "extract_failure_cluster_principle" {
        let input: ExtractFailureClusterPrincipleInput =
            decode_input(&payload, "extract_failure_cluster_principle_input")?;
        let out = compute_extract_failure_cluster_principle(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "extract_failure_cluster_principle",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_extract_failure_cluster_principle_failed:{e}"));
    }
    if mode == "persist_first_principle" {
        let input: PersistFirstPrincipleInput =
            decode_input(&payload, "persist_first_principle_input")?;
        let out = compute_persist_first_principle(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "persist_first_principle",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_persist_first_principle_failed:{e}"));
    }
    Err(format!(
        "inversion_mode_unsupported:raw={mode_raw}:normalized={mode}"
    ))
}
