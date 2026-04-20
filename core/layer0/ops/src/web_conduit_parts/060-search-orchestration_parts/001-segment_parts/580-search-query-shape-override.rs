fn search_query_shape_override(policy: &Value, request: &Value) -> bool {
    for key in [
        "/allow_query_blob_search",
        "/allowQueryBlobSearch",
        "/allow_query_shape_override",
        "/allowQueryShapeOverride",
        "/force_query_shape_override",
        "/forceQueryShapeOverride",
    ] {
        if let Some(value) = request.pointer(key) {
            if search_truthy_value(value) {
                return true;
            }
        }
    }
    for key in [
        "/web_conduit/search_policy/allow_query_blob_search",
        "/web_conduit/search_policy/allowQueryBlobSearch",
        "/web_conduit/search_policy/allow_query_shape_override",
        "/web_conduit/search_policy/allowQueryShapeOverride",
        "/web_conduit/search_policy/force_query_shape_override",
        "/web_conduit/search_policy/forceQueryShapeOverride",
    ] {
        if let Some(value) = policy.pointer(key) {
            if search_truthy_value(value) {
                return true;
            }
        }
    }
    false
}
