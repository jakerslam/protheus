    if let Some(receipt_obj) = receipt.as_object_mut() {
        receipt_obj.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature),
        );
        receipt_obj.insert(
            "provider".to_string(),
            Value::String(final_selected_provider.clone()),
        );
    }
    let _ = append_jsonl(&receipts_path(root), &receipt);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("receipt".to_string(), receipt);
    }
    if cache_ttl_seconds > 0 && !challenge_like_failure {
        store_search_cache(
            root,
            &cache_key,
            &out,
            cache_status,
            Some(cache_ttl_seconds),
        );
    }
    out
