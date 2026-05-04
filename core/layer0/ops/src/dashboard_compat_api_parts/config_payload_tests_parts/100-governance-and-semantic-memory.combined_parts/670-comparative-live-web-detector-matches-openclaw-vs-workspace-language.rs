fn comparative_language_does_not_select_live_web_detector() {
    let candidates = latent_tool_candidates_for_message(
        "compare openclaw to this system/workspace using web search",
        &[],
    );
    assert!(candidates.is_empty(), "{candidates:?}");
}

#[test]
