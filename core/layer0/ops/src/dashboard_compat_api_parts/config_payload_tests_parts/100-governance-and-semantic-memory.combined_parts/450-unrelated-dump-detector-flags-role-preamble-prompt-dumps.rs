fn unrelated_dump_detector_flags_role_preamble_prompt_dumps() {
    let dump = "I am an expert in the field of deep learning, neural networks, and AI ethics. My role is to provide clear, accurate explanations while maintaining a professional tone. The user has provided a detailed draft response, and my task is to refine and finalize it based on workflow metadata. Source: The Model's Training Data. Mechanism: Faulty Pattern Retrieval. The Error: Context Collapse.";
    assert!(response_is_unrelated_context_dump(
        "but where did the hallucination come from?",
        dump
    ));
}

#[test]
