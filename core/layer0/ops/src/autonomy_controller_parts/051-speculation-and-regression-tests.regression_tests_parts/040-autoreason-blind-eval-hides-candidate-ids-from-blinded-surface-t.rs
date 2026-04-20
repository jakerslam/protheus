
#[test]
fn autoreason_blind_eval_hides_candidate_ids_from_blinded_surface() {
    let eval = autoreason_blind_evaluate(
        "ar-test",
        1,
        &[
            ("a_revised".to_string(), "candidate a body".to_string()),
            ("b_revised".to_string(), "candidate b body".to_string()),
            ("ab_synth".to_string(), "candidate ab body".to_string()),
        ],
        3,
    );
    let blinded = eval
        .get("blinded_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!blinded.is_empty());
    assert!(blinded.iter().all(|row| row.get("candidate_id").is_none()));
    let winner = eval.get("winner_id").and_then(Value::as_str).unwrap_or("");
    assert!(matches!(winner, "a_revised" | "b_revised" | "ab_synth"));
}

#[test]
fn autoreason_conduit_bypass_is_rejected() {
    let root = tempdir().expect("tmp");
    assert_eq!(
        run_autoreason(
            root.path(),
            &[
                "autoreason".to_string(),
                "run".to_string(),
                "--task=t".to_string(),
                "--bypass=1".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        1
    );
}
