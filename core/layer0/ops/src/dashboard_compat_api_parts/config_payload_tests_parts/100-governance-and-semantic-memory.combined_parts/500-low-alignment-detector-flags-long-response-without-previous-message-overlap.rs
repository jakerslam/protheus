fn low_alignment_detector_flags_long_response_without_previous_message_overlap() {
    let user_message =
        "Well give me some actionable steps cause those were really broad. Give 10 steps";
    let recent_context =
        "We are discussing web tooling reliability and context retention in final responses.";
    let unrelated_long_response = "03-树2 List Leaves (25 分) Given a tree, you are supposed to list all the leaves in the order of top down, and left to right. Input Specification: Each input file contains one test case. For each case, the first line gives a positive integer N (≤10). Sample Input: 8. Sample Output: 4 1 5.";
    assert!(response_low_alignment_with_turn_context(
        user_message,
        recent_context,
        unrelated_long_response
    ));
}

#[test]
