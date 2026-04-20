        let mut ledger = load_ledger(root);
        if let Some(obj) = ledger.as_object_mut() {
            let claims = map_mut(obj, "zk_claims");
            claims.insert(
                claim_id.clone(),
                json!({
                    "commitment": commitment,
                    "challenge": challenge,
                    "public_input": public_input,
                    "expected_challenge": expected_challenge,
                    "verified": verified,
                    "strict": strict,
                    "ts": now_iso()
                }),
            );
        }

        let updated = commit_ledger(
            root,
            ledger,
            "zk_claim",
            json!({"claim_id": claim_id, "verified": verified, "strict": strict}),
        );
        match updated {
            Ok(ledger2) => emit(
                root,
                json!({
                    "ok": ok,
                    "type": "network_protocol_zk_claim",
                    "lane": "core/layer0/ops",
                    "claim_id": claim_id,
                    "verified": verified,
                    "strict": strict,
                    "expected_challenge": expected_challenge,
                    "network_state_root": ledger2.get("root_head").cloned().unwrap_or(Value::Null),
                    "layer_map": ["0","1","2","adapter"],
                    "claim_evidence": [
                        {
                            "id": "V8-NETWORK-002.4",
                            "claim": "private_claim_verification_is_policy_gated_and_receipted",
                            "evidence": {"claim_id": claim_id, "verified": verified, "strict": strict}
                        }
                    ]
                }),
            ),
            Err(err) => emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_zk_claim",
                    "lane": "core/layer0/ops",
                    "error": clean(err, 220)
                }),
            ),
        }
    } else {
        emit(
            root,
            json!({
                "ok": false,
                "type": "network_protocol_error",
                "lane": "core/layer0/ops",
                "error": "unknown_command",
                "command": command,
                "exit_code": 2
            }),
        )
    }
}
