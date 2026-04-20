        let mut errors = Vec::<String>::new();
        if strict && admission_token.len() < 8 {
            errors.push("network_admission_token_too_short".to_string());
        }
        let mut membership = read_json(&membership_path(root)).unwrap_or_else(|| {
            json!({
                "schema_id": "network_membership_v1",
                "nodes": []
            })
        });
        let mut nodes = membership
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if errors.is_empty() {
            let mut found = false;
            for row in &mut nodes {
                if row.get("id").and_then(Value::as_str) == Some(node.as_str()) {
                    row["joined_at"] = Value::String(now_iso());
                    row["admitted"] = Value::Bool(true);
                    row["stake"] = Value::from(stake);
                    found = true;
                    break;
                }
            }
            if !found {
                nodes.push(json!({
                    "id": node,
                    "admitted": true,
                    "stake": stake,
                    "reputation_weight": (1.0 + (stake / 100.0)).min(10.0),
                    "joined_at": now_iso()
                }));
            }
            membership["nodes"] = Value::Array(nodes.clone());
            let _ = write_json(&membership_path(root), &membership);
        }
        emit(
            root,
            json!({
                "ok": if strict { errors.is_empty() } else { true },
                "type": "network_protocol_join_hyperspace",
                "lane": "core/layer0/ops",
                "strict": strict,
                "profile": "hyperspace",
                "node": node,
                "stake": stake,
                "member_count": nodes.len(),
                "errors": errors,
                "claim_evidence": [
                    {
                        "id": "V7-NETWORK-001.4",
                        "claim": "secure_network_join_records_membership_and_reputation_weighted_governance_state",
                        "evidence": {"node": node, "member_count": nodes.len()}
                    }
                ]
            }),
        )
    } else if command == "governance-vote" {
        let strict = parse_bool(parsed.flags.get("strict"), true);
        let proposal = clean(
            parsed
                .flags
                .get("proposal")
                .cloned()
                .unwrap_or_else(|| "proposal-default".to_string()),
            160,
        );
        let voter = clean(
            parsed
                .flags
                .get("voter")
                .cloned()
                .unwrap_or_else(|| "node-local".to_string()),
            120,
        );
        let vote = clean(
            parsed
                .flags
                .get("vote")
                .cloned()
                .unwrap_or_else(|| "approve".to_string()),
            20,
        )
        .to_ascii_lowercase();
        let membership = read_json(&membership_path(root)).unwrap_or_else(|| json!({"nodes": []}));
        let voter_row = membership
            .get("nodes")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row.get("id").and_then(Value::as_str) == Some(voter.as_str()))
            })
            .cloned()
            .unwrap_or_else(|| json!({"id": voter, "reputation_weight": 1.0}));
        let weight = voter_row
            .get("reputation_weight")
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            .max(0.1);
        let mut errors = Vec::<String>::new();
        if strict && !matches!(vote.as_str(), "approve" | "reject") {
            errors.push("governance_vote_invalid".to_string());
        }
        let vote_event = json!({
            "proposal": proposal,
            "voter": voter,
            "vote": vote,
            "weight": weight,
            "ts": now_iso()
        });
        if errors.is_empty() {
            let _ = append_jsonl(&governance_votes_path(root), &vote_event);
        }
        let all_votes = read_jsonl(&governance_votes_path(root));
        let approve_weight = all_votes
            .iter()
            .filter(|row| row.get("proposal").and_then(Value::as_str) == Some(proposal.as_str()))
            .filter(|row| row.get("vote").and_then(Value::as_str) == Some("approve"))
            .filter_map(|row| row.get("weight").and_then(Value::as_f64))
            .sum::<f64>();
        let reject_weight = all_votes
            .iter()
            .filter(|row| row.get("proposal").and_then(Value::as_str) == Some(proposal.as_str()))
            .filter(|row| row.get("vote").and_then(Value::as_str) == Some("reject"))
            .filter_map(|row| row.get("weight").and_then(Value::as_f64))
            .sum::<f64>();
        emit(
            root,
            json!({
                "ok": if strict { errors.is_empty() } else { true },
                "type": "network_protocol_governance_vote",
                "lane": "core/layer0/ops",
                "strict": strict,
                "event": vote_event,
                "tally": {
                    "approve_weight": approve_weight,
                    "reject_weight": reject_weight
                },
                "errors": errors,
                "claim_evidence": [
                    {
                        "id": "V7-NETWORK-001.4",
                        "claim": "governance_actions_are_reputation_weighted_and_receipted",
                        "evidence": {"proposal": proposal, "approve_weight": approve_weight, "reject_weight": reject_weight}
                    }
                ]
            }),
        )
    } else if command == "oracle-query" {
        let provider = clean(
            parsed
                .flags
                .get("provider")
                .cloned()
                .unwrap_or_else(|| "polymarket".to_string()),
            80,
        )
        .to_ascii_lowercase();
        let event = clean(
            parsed
                .flags
                .get("event")
                .cloned()
                .unwrap_or_else(|| "default-event".to_string()),
            240,
        );
        let strict = parse_bool(parsed.flags.get("strict"), true);
        let gate_ok = gate_action(root, &format!("oracle:query:{provider}:{event}"));
        if strict && !gate_ok {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_oracle_query",
                    "lane": "core/layer0/ops",
                    "provider": provider,
                    "event": event,
                    "error": "directive_gate_denied",
