fn unrelated_dump_detector_flags_dataframe_instruction_template_dump() {
    let dump = "1. Find the 10 countries with most projects #The information about the countries is contained in the 'countryname' column of the dataframe\ndf_json['countryname'].value_counts().head(10)";
    assert!(response_is_unrelated_context_dump(
        "how can we find out if it was an actual tool call error or llm error",
        dump
    ));
}

#[test]
