fn unrelated_dump_detector_flags_reddit_goldbach_thread_dump() {
    let dump = "[Request] Is this even possible? How?\n9k Upvotes\nThere is a mathematical proof that shows that all even numbers >=4 can be expressed as the sum of two primes. This is known as the Goldbach Conjecture.";
    assert!(response_is_unrelated_context_dump(
        "try searching for information about the top agentic frameworks for me",
        dump
    ));
}

#[test]
