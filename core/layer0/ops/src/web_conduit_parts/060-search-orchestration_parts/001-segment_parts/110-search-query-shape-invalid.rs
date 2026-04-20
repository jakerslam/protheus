fn search_query_shape_invalid(query: &str) -> bool {
    search_query_shape_error_code(query) != "none"
}
