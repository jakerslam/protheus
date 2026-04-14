use super::*;

#[test]
fn envelope_serializes_and_validates() {
    let env = SwarmEnvelope::new_auto("coord", "lane/review", json!({"x":1}), 5);
    env.validate().expect("envelope should validate");
    let encoded = serde_json::to_string(&env).expect("encode envelope");
    let decoded: SwarmEnvelope = serde_json::from_str(&encoded).expect("decode envelope");
    assert_eq!(decoded.route, "lane/review");
    assert_eq!(decoded.priority, 5);
}

#[test]
fn auto_id_is_parseable_and_collision_safe() {
    let a = auto_id("worker");
    let b = auto_id("worker");
    assert_ne!(a, b);
    assert!(a.starts_with("worker-"));
    assert_eq!(a.split('-').count(), 2);
}

#[test]
fn in_flight_tracker_blocks_conflicts() {
    let mut tracker = InFlightTracker::default();
    let env = SwarmEnvelope::new_auto("coord", "lane/a", json!({}), 3);
    tracker.dispatch(&env, "worker-1").expect("first dispatch");
    let err = tracker
        .dispatch(&env, "worker-2")
        .expect_err("conflict expected");
    assert_eq!(err, "in_flight_conflict");
    tracker
        .transition(&env.id, TaskStatus::Complete)
        .expect("complete task");
    tracker
        .dispatch(&env, "worker-2")
        .expect("redispatch after completion");
}

#[test]
fn recovery_routes_retry_then_fixer() {
    let env = SwarmEnvelope::new_auto("worker", "lane/x", json!({}), 4);
    let policy = RecoveryPolicy {
        max_retries: 2,
        fixer_route: "lane/fixer".to_string(),
    };
    assert_eq!(
        recovery_decision(&env, 0, &policy),
        RecoveryDecision::Retry {
            attempt: 1,
            route: "lane/x".to_string()
        }
    );
    assert_eq!(
        recovery_decision(&env, 2, &policy),
        RecoveryDecision::RerouteFixer {
            route: "lane/fixer".to_string()
        }
    );
}

#[test]
fn scaling_policy_recommends_changes() {
    let policy = ScalingPolicy {
        min_workers: 1,
        max_workers: 8,
        target_queue_per_worker: 3,
        scale_step: 2,
    };
    let up = plan_scaling(20, 2, &policy);
    assert_eq!(up.action, "scale_up");
    assert!(up.recommended_workers > up.previous_workers);

    let down = plan_scaling(1, 4, &policy);
    assert_eq!(down.action, "scale_down");
    assert!(down.recommended_workers < down.previous_workers);
}

#[test]
fn queue_contract_persists_schema_and_priority_order() {
    let tmp = std::env::temp_dir().join(format!("swarm_router_queue_{}.json", auto_id("test")));
    let mut queue = QueueArtifact::default();
    let mut low = SwarmEnvelope::new_auto("coord", "lane/low", json!({"k":"low"}), 2);
    low.id = "coord-0000000000000002".to_string();
    let mut high = SwarmEnvelope::new_auto("coord", "lane/high", json!({"k":"high"}), 8);
    high.id = "coord-0000000000000001".to_string();
    let mut tie = SwarmEnvelope::new_auto("coord", "lane/tie", json!({"k":"tie"}), 8);
    tie.id = "coord-0000000000000003".to_string();

    queue.push(low).expect("push low");
    queue.push(tie).expect("push tie");
    queue.push(high).expect("push high");
    queue.save(&tmp).expect("save queue");

    let loaded = QueueArtifact::load(&tmp).expect("load queue");
    assert_eq!(loaded.items[0].id, "coord-0000000000000001");
    assert_eq!(loaded.items[1].id, "coord-0000000000000003");
    assert_eq!(loaded.items[2].id, "coord-0000000000000002");

    let _ = std::fs::remove_file(tmp);
}

#[test]
fn observability_and_upgrade_receipts_are_deterministic_shape() {
    let metrics = build_metrics(10, 2, 3, 8, 5);
    assert_eq!(metrics.queue_depth, 5);
    assert!(metrics.fail_rate > 0.0);
    let env = SwarmEnvelope::new_auto("coord", "lane/a", json!({"x":1}), 5);
    let receipt = build_receipt(
        "dispatch",
        Some(&env),
        json!({"owner":"worker-1"}),
        Some(metrics.clone()),
    );
    assert_eq!(receipt.schema_id, "swarm_router_receipt_v1");
    assert_eq!(receipt.envelope_id.as_deref(), Some(env.id.as_str()));
    assert_eq!(receipt.metrics.expect("metrics").retry_count, 3);

    let policy = UpgradePolicy::default();
    let up = apply_upgrade("1.0.0", "1.1.0", &policy);
    assert!(up.ok);
    assert!(up.rollback_available);

    let rb = apply_rollback("1.1.0", "1.0.0", &policy);
    assert!(rb.ok);
    assert_eq!(rb.action, "rollback_applied");
}
