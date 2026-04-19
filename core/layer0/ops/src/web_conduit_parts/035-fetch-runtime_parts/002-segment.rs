fn fetch_meta_query_override(request: &Value) -> bool {
    let pointers = [
        "/allow_meta_query_search",
        "/allowMetaQuerySearch",
        "/force_web_search",
        "/forceWebSearch",
        "/force_web_lookup",
        "/forceWebLookup",
        "/allow_meta_query_fetch",
        "/allowMetaQueryFetch",
        "/force_web_fetch",
        "/forceWebFetch",
        "/search_policy/allow_meta_query_search",
        "/searchPolicy/allowMetaQuerySearch",
        "/search_policy/force_web_search",
        "/searchPolicy/forceWebSearch",
        "/search_policy/force_web_lookup",
        "/searchPolicy/forceWebLookup",
        "/fetch_policy/allow_meta_query_fetch",
        "/fetchPolicy/allowMetaQueryFetch",
        "/fetch_policy/force_web_fetch",
        "/fetchPolicy/forceWebFetch",
    ];
    runtime_web_request_flag(request, &pointers)
}
