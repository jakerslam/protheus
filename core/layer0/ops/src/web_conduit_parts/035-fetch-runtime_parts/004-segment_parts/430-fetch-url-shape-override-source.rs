fn fetch_url_shape_override_source(policy: &Value, request: &Value) -> &'static str {
    for key in [
        "/allow_fetch_url_blob",
        "/allowFetchUrlBlob",
        "/allow_fetch_url_shape_override",
        "/allowFetchUrlShapeOverride",
        "/force_fetch_url_shape_override",
        "/forceFetchUrlShapeOverride",
        "/force_web_fetch",
        "/forceWebFetch",
    ] {
        if let Some(value) = request.pointer(key) {
            if fetch_truthy_value(value) {
                return "request";
            }
        }
    }
    for key in [
        "/web_conduit/fetch_policy/allow_fetch_url_blob",
        "/web_conduit/fetch_policy/allowFetchUrlBlob",
        "/web_conduit/fetch_policy/allow_fetch_url_shape_override",
        "/web_conduit/fetch_policy/allowFetchUrlShapeOverride",
        "/web_conduit/fetch_policy/force_fetch_url_shape_override",
        "/web_conduit/fetch_policy/forceFetchUrlShapeOverride",
    ] {
        if let Some(value) = policy.pointer(key) {
            if fetch_truthy_value(value) {
                return "policy";
            }
        }
    }
    "none"
}
