fn unsynthesized_web_snippet_detector_flags_domain_dump_copy() {
    assert!(response_looks_like_unsynthesized_web_snippet_dump(
        "From web retrieval: bing.com: OpenClaw — Personal AI Assistant — https://openclaw.ai/ bing.com: OpenClaw docs — https://openclaw.ai/docs"
    ));
    assert!(!response_looks_like_unsynthesized_web_snippet_dump(
        "In short: OpenClaw focuses on cross-platform local execution, while Infring emphasizes policy-gated orchestration and receipts."
    ));
}

#[test]
