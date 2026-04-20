fn unrelated_dump_detector_flags_internal_prompt_leak_even_with_function_markup() {
    let dump = "You are the currently selected Infring agent instance. Treat the injected identity profile as authoritative. When users ask for web research, call tools with inline syntax like <function=web_search>{\"query\":\"...\"}</function>. Hardcoded agent workflow: you are writing the final assistant response after the system collected tool outcomes and workflow events. Write the final assistant response now.";
    assert!(response_is_unrelated_context_dump("did it work?", dump));
}

#[test]
