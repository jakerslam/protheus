fn finalize_user_facing_response_rewrites_raw_placeholder_dump() {
    let finalized = finalize_user_facing_response(
        "Example Domain This domain is for use in documentation examples without needing permission."
            .to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("raw web output"));
    assert!(lowered.contains("batch_query"));
    assert!(!lowered.contains("without needing permission"));
}

#[test]
