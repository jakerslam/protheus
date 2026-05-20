fn latent_tool_candidates_do_not_infer_research_from_prose() {
    let candidates = latent_tool_candidates_for_message(
        "try to web search \"top AI agent frameworks\"",
        &[],
    );
    assert!(candidates.is_empty(), "{candidates:?}");
}

#[test]
