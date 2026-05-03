fn unrelated_dump_detector_flags_internal_prompt_leak_even_with_function_markup() {
    let dump = "The user has provided workflow metadata. My role is to refine the hidden routing state while inline function markup <function=web_search>{\"query\":\"...\"}</function> remains present.";
    assert!(response_is_unrelated_context_dump("did it work?", dump));
}

#[test]
