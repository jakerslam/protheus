mod tests {
    use super::*;

    #[test]
    fn normalize_campaigns_filters_inactive_and_sorts_phases() {
        let strategy = json!({
            "campaigns": [
                {
                    "id": "Campaign-A",
                    "status": "active",
                    "priority": 20,
                    "phases": [
                        {"id": "phase-b", "status": "active", "order": 2, "priority": 1},
                        {"id": "phase-a", "status": "active", "order": 1, "priority": 2},
                        {"id": "phase-z", "status": "paused", "order": 0}
                    ]
                },
                {
                    "id": "Campaign-B",
                    "status": "paused",
                    "phases": [{"id": "phase-x", "status": "active"}]
                }
            ]
        });
        let campaigns = normalize_campaigns(&strategy);
        assert_eq!(campaigns.len(), 1);
        assert_eq!(campaigns[0].phases.len(), 2);
        assert_eq!(campaigns[0].phases[0].id, "phase-a");
    }

    #[test]
    fn decomposition_respects_existing_seed_and_open_counts() {
        let strategy = json!({
            "campaigns": [{
                "id": "Campaign-A",
                "status": "active",
                "priority": 20,
                "objective_id": "OBJ-1",
                "phases": [{
                    "id": "phase-a",
                    "status": "active",
                    "proposal_types": ["fix"]
                }]
            }]
        });
        let proposals = vec![json!({
            "id": "CAMP-CAMPAIGN-A-PHASE-A-FIX-OBJ-1",
            "type": "fix",
            "meta": {
                "campaign_seed_key": "campaign-a|phase-a|fix|obj-1"
            }
        })];
        let out = build_campaign_decomposition_plans(
            &proposals,
            &strategy,
            payload_obj(&json!({"max_additions": 1, "min_open_per_type": 1})),
        );
        assert_eq!(
            out.get("additions")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(0)
        );
    }

    #[test]
    fn annotate_priority_prefers_phase_specific_proposal_type_filter() {
        let strategy = json!({
            "campaigns": [{
                "id": "Objective Flow",
                "name": "Objective Flow",
                "status": "active",
                "priority": 20,
                "objective_id": "OBJ-1",
                "proposal_types": ["strategy"],
                "phases": [{
                    "id": "stabilize",
                    "name": "stabilize",
                    "status": "active",
                    "order": 1,
                    "priority": 10,
                    "proposal_types": ["infrastructure_outage"],
                    "source_eyes": ["health"],
                    "tags": ["ops"]
                }]
            }]
        });
        let candidates = vec![json!({
            "proposal": {
                "type": "infrastructure_outage",
                "meta": { "source_eye": "health", "objective_id": "OBJ-1", "tags": ["ops"] },
                "tags": ["ops"]
            }
        })];
        let out = annotate_campaign_priority(&candidates, &strategy);
        assert_eq!(
            out.pointer("/summary/matched_count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            out.pointer("/candidates/0/campaign_match/campaign_id")
                .and_then(Value::as_str),
            Some("objective flow")
        );
    }
}
