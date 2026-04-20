fn fetch_url_source_and_input(request: &Value) -> (String, &'static str) {
    for (key, source) in [
        ("requested_url", "requested_url"),
        ("url", "url"),
        ("target", "target"),
        ("link", "link"),
        ("requestedUrl", "requestedUrl"),
        ("targetUrl", "targetUrl"),
        ("sourceUrl", "sourceUrl"),
        ("target_url", "target_url"),
        ("href", "href"),
        ("uri", "uri"),
    ] {
        let candidate = clean_text(
            request.get(key).and_then(Value::as_str).unwrap_or(""),
            2200,
        );
        if !candidate.trim().is_empty() {
            return (candidate, source);
        }
    }
    for (pointer, source) in [
        ("/request/requested_url", "request.requested_url"),
        ("/request/url", "request.url"),
        ("/request/target", "request.target"),
        ("/request/link", "request.link"),
        ("/request/data/requested_url", "request.data.requested_url"),
        ("/request/data/url", "request.data.url"),
        ("/request/data/target", "request.data.target"),
        ("/request/data/link", "request.data.link"),
        ("/request/body/requested_url", "request.body.requested_url"),
        ("/request/body/url", "request.body.url"),
        ("/request/body/target", "request.body.target"),
        ("/request/body/link", "request.body.link"),
        ("/request/requestedUrl", "request.requestedUrl"),
        ("/request/targetUrl", "request.targetUrl"),
        ("/request/sourceUrl", "request.sourceUrl"),
        ("/request/data/requestedUrl", "request.data.requestedUrl"),
        ("/request/data/targetUrl", "request.data.targetUrl"),
        ("/request/data/sourceUrl", "request.data.sourceUrl"),
        ("/request/body/requestedUrl", "request.body.requestedUrl"),
        ("/request/body/targetUrl", "request.body.targetUrl"),
        ("/request/body/sourceUrl", "request.body.sourceUrl"),
        ("/request/target_url", "request.target_url"),
        ("/request/href", "request.href"),
        ("/request/uri", "request.uri"),
        ("/request/data/target_url", "request.data.target_url"),
        ("/request/data/href", "request.data.href"),
        ("/request/data/uri", "request.data.uri"),
        ("/request/body/target_url", "request.body.target_url"),
        ("/request/body/href", "request.body.href"),
        ("/request/body/uri", "request.body.uri"),
    ] {
        let candidate = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 2200);
        if !candidate.trim().is_empty() {
            return (candidate, source);
        }
    }
    for (pointer, source) in [
        ("/request/query", "request.query"),
        ("/request/q", "request.q"),
        ("/request/search_query", "request.search_query"),
        ("/request/searchQuery", "request.searchQuery"),
        ("/request/data/query", "request.data.query"),
        ("/request/data/q", "request.data.q"),
        ("/request/data/search_query", "request.data.search_query"),
        ("/request/data/searchQuery", "request.data.searchQuery"),
        ("/request/body/query", "request.body.query"),
        ("/request/body/q", "request.body.q"),
        ("/request/body/search_query", "request.body.search_query"),
        ("/request/body/searchQuery", "request.body.searchQuery"),
    ] {
        let candidate = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 2200);
        if let Some(url_candidate) = fetch_url_candidate_from_text(&candidate) {
            return (url_candidate, source);
        }
    }
    for (pointer, source) in [
        ("/payload/requested_url", "payload.requested_url"),
        ("/payload/url", "payload.url"),
        ("/payload/target", "payload.target"),
        ("/payload/link", "payload.link"),
        ("/payload/data/requested_url", "payload.data.requested_url"),
        ("/payload/data/url", "payload.data.url"),
        ("/payload/data/target", "payload.data.target"),
        ("/payload/data/link", "payload.data.link"),
        ("/payload/body/requested_url", "payload.body.requested_url"),
        ("/payload/body/url", "payload.body.url"),
        ("/payload/body/target", "payload.body.target"),
        ("/payload/body/link", "payload.body.link"),
        ("/payload/requestedUrl", "payload.requestedUrl"),
        ("/payload/targetUrl", "payload.targetUrl"),
        ("/payload/sourceUrl", "payload.sourceUrl"),
        ("/payload/data/requestedUrl", "payload.data.requestedUrl"),
        ("/payload/data/targetUrl", "payload.data.targetUrl"),
        ("/payload/data/sourceUrl", "payload.data.sourceUrl"),
        ("/payload/body/requestedUrl", "payload.body.requestedUrl"),
        ("/payload/body/targetUrl", "payload.body.targetUrl"),
        ("/payload/body/sourceUrl", "payload.body.sourceUrl"),
        ("/payload/target_url", "payload.target_url"),
        ("/payload/href", "payload.href"),
        ("/payload/uri", "payload.uri"),
        ("/payload/data/target_url", "payload.data.target_url"),
        ("/payload/data/href", "payload.data.href"),
        ("/payload/data/uri", "payload.data.uri"),
        ("/payload/body/target_url", "payload.body.target_url"),
        ("/payload/body/href", "payload.body.href"),
        ("/payload/body/uri", "payload.body.uri"),
    ] {
        let candidate = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 2200);
        if !candidate.trim().is_empty() {
            return (candidate, source);
        }
    }
    for (pointer, source) in [
        ("/payload/request/requested_url", "payload.request.requested_url"),
        ("/payload/request/url", "payload.request.url"),
        ("/payload/request/target", "payload.request.target"),
        ("/payload/request/link", "payload.request.link"),
        (
            "/payload/request/data/requested_url",
            "payload.request.data.requested_url",
        ),
        ("/payload/request/data/url", "payload.request.data.url"),
        ("/payload/request/data/target", "payload.request.data.target"),
        ("/payload/request/data/link", "payload.request.data.link"),
        ("/payload/request/body/requested_url", "payload.request.body.requested_url"),
        ("/payload/request/body/url", "payload.request.body.url"),
        ("/payload/request/body/target", "payload.request.body.target"),
        ("/payload/request/body/link", "payload.request.body.link"),
        ("/payload/request/requestedUrl", "payload.request.requestedUrl"),
        ("/payload/request/targetUrl", "payload.request.targetUrl"),
        ("/payload/request/sourceUrl", "payload.request.sourceUrl"),
        (
            "/payload/request/data/requestedUrl",
            "payload.request.data.requestedUrl",
        ),
        (
            "/payload/request/data/targetUrl",
            "payload.request.data.targetUrl",
        ),
        (
            "/payload/request/data/sourceUrl",
            "payload.request.data.sourceUrl",
        ),
        ("/payload/request/body/requestedUrl", "payload.request.body.requestedUrl"),
        ("/payload/request/body/targetUrl", "payload.request.body.targetUrl"),
        ("/payload/request/body/sourceUrl", "payload.request.body.sourceUrl"),
        ("/payload/request/target_url", "payload.request.target_url"),
        ("/payload/request/href", "payload.request.href"),
        ("/payload/request/uri", "payload.request.uri"),
        (
            "/payload/request/data/target_url",
            "payload.request.data.target_url",
        ),
        ("/payload/request/data/href", "payload.request.data.href"),
        ("/payload/request/data/uri", "payload.request.data.uri"),
        ("/payload/request/body/target_url", "payload.request.body.target_url"),
        ("/payload/request/body/href", "payload.request.body.href"),
        ("/payload/request/body/uri", "payload.request.body.uri"),
    ] {
        let candidate = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 2200);
        if !candidate.trim().is_empty() {
            return (candidate, source);
        }
    }
    for (pointer, source) in [
        ("/payload/request/query", "payload.request.query"),
        ("/payload/request/q", "payload.request.q"),
        ("/payload/request/search_query", "payload.request.search_query"),
        ("/payload/request/searchQuery", "payload.request.searchQuery"),
        ("/payload/request/data/query", "payload.request.data.query"),
        ("/payload/request/data/q", "payload.request.data.q"),
        (
            "/payload/request/data/search_query",
            "payload.request.data.search_query",
        ),
        (
            "/payload/request/data/searchQuery",
            "payload.request.data.searchQuery",
        ),
        ("/payload/request/body/query", "payload.request.body.query"),
        ("/payload/request/body/q", "payload.request.body.q"),
        ("/payload/request/body/search_query", "payload.request.body.search_query"),
        ("/payload/request/body/searchQuery", "payload.request.body.searchQuery"),
    ] {
        let candidate = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 2200);
        if let Some(url_candidate) = fetch_url_candidate_from_text(&candidate) {
            return (url_candidate, source);
        }
    }
    for (pointer, source) in [
        ("/payload/query", "payload.query"),
        ("/payload/q", "payload.q"),
        ("/payload/search_query", "payload.search_query"),
        ("/payload/searchQuery", "payload.searchQuery"),
        ("/payload/data/query", "payload.data.query"),
        ("/payload/data/q", "payload.data.q"),
        ("/payload/data/search_query", "payload.data.search_query"),
        ("/payload/data/searchQuery", "payload.data.searchQuery"),
        ("/payload/body/query", "payload.body.query"),
        ("/payload/body/q", "payload.body.q"),
        ("/payload/body/search_query", "payload.body.search_query"),
        ("/payload/body/searchQuery", "payload.body.searchQuery"),
    ] {
        let candidate = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 2200);
        if let Some(url_candidate) = fetch_url_candidate_from_text(&candidate) {
            return (url_candidate, source);
        }
    }
    for (pointer, source) in [
        ("/urls/0", "urls[0]"),
        ("/request/urls/0", "request.urls[0]"),
        ("/request/data/urls/0", "request.data.urls[0]"),
        ("/payload/urls/0", "payload.urls[0]"),
        ("/payload/data/urls/0", "payload.data.urls[0]"),
        ("/payload/request/urls/0", "payload.request.urls[0]"),
        ("/payload/request/data/urls/0", "payload.request.data.urls[0]"),
        ("/request/body/urls/0", "request.body.urls[0]"),
        ("/payload/body/urls/0", "payload.body.urls[0]"),
        ("/payload/request/body/urls/0", "payload.request.body.urls[0]"),
    ] {
        let candidate = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 2200);
        if let Some(url_candidate) = fetch_url_candidate_from_text(&candidate) {
            return (url_candidate, source);
        }
    }
    for (pointer, source_prefix) in [
        ("/urls/0", "urls[0]"),
        ("/request/urls/0", "request.urls[0]"),
        ("/request/data/urls/0", "request.data.urls[0]"),
        ("/payload/urls/0", "payload.urls[0]"),
        ("/payload/data/urls/0", "payload.data.urls[0]"),
        ("/payload/request/urls/0", "payload.request.urls[0]"),
        ("/payload/request/data/urls/0", "payload.request.data.urls[0]"),
        ("/request/body/urls/0", "request.body.urls[0]"),
        ("/payload/body/urls/0", "payload.body.urls[0]"),
        ("/payload/request/body/urls/0", "payload.request.body.urls[0]"),
    ] {
        for (field, source_suffix) in [
            ("url", ".url"),
            ("href", ".href"),
            ("uri", ".uri"),
            ("link", ".link"),
            ("target", ".target"),
        ] {
            let pointer_with_field = format!("{}/{}", pointer, field);
            let candidate = clean_text(
                request.pointer(&pointer_with_field).and_then(Value::as_str).unwrap_or(""),
                2200,
            );
            if let Some(url_candidate) = fetch_url_candidate_from_text(&candidate) {
                let source_name = match (source_prefix, source_suffix) {
                    ("urls[0]", ".url") => "urls[0].url",
                    ("urls[0]", ".href") => "urls[0].href",
                    ("urls[0]", ".uri") => "urls[0].uri",
                    ("urls[0]", ".link") => "urls[0].link",
                    ("urls[0]", ".target") => "urls[0].target",
                    ("request.urls[0]", ".url") => "request.urls[0].url",
                    ("request.urls[0]", ".href") => "request.urls[0].href",
                    ("request.urls[0]", ".uri") => "request.urls[0].uri",
                    ("request.urls[0]", ".link") => "request.urls[0].link",
                    ("request.urls[0]", ".target") => "request.urls[0].target",
                    ("request.data.urls[0]", ".url") => "request.data.urls[0].url",
                    ("request.data.urls[0]", ".href") => "request.data.urls[0].href",
                    ("request.data.urls[0]", ".uri") => "request.data.urls[0].uri",
                    ("request.data.urls[0]", ".link") => "request.data.urls[0].link",
                    ("request.data.urls[0]", ".target") => "request.data.urls[0].target",
                    ("payload.urls[0]", ".url") => "payload.urls[0].url",
                    ("payload.urls[0]", ".href") => "payload.urls[0].href",
                    ("payload.urls[0]", ".uri") => "payload.urls[0].uri",
                    ("payload.urls[0]", ".link") => "payload.urls[0].link",
                    ("payload.urls[0]", ".target") => "payload.urls[0].target",
                    ("payload.data.urls[0]", ".url") => "payload.data.urls[0].url",
                    ("payload.data.urls[0]", ".href") => "payload.data.urls[0].href",
                    ("payload.data.urls[0]", ".uri") => "payload.data.urls[0].uri",
                    ("payload.data.urls[0]", ".link") => "payload.data.urls[0].link",
                    ("payload.data.urls[0]", ".target") => "payload.data.urls[0].target",
                    ("payload.request.urls[0]", ".url") => "payload.request.urls[0].url",
                    ("payload.request.urls[0]", ".href") => "payload.request.urls[0].href",
                    ("payload.request.urls[0]", ".uri") => "payload.request.urls[0].uri",
                    ("payload.request.urls[0]", ".link") => "payload.request.urls[0].link",
                    ("payload.request.urls[0]", ".target") => "payload.request.urls[0].target",
                    ("payload.request.data.urls[0]", ".url") => "payload.request.data.urls[0].url",
                    ("payload.request.data.urls[0]", ".href") => "payload.request.data.urls[0].href",
                    ("payload.request.data.urls[0]", ".uri") => "payload.request.data.urls[0].uri",
                    ("payload.request.data.urls[0]", ".link") => "payload.request.data.urls[0].link",
                    ("payload.request.data.urls[0]", ".target") => "payload.request.data.urls[0].target",
                    ("request.body.urls[0]", ".url") => "request.body.urls[0].url",
                    ("request.body.urls[0]", ".href") => "request.body.urls[0].href",
                    ("request.body.urls[0]", ".uri") => "request.body.urls[0].uri",
                    ("request.body.urls[0]", ".link") => "request.body.urls[0].link",
                    ("request.body.urls[0]", ".target") => "request.body.urls[0].target",
                    ("payload.body.urls[0]", ".url") => "payload.body.urls[0].url",
                    ("payload.body.urls[0]", ".href") => "payload.body.urls[0].href",
                    ("payload.body.urls[0]", ".uri") => "payload.body.urls[0].uri",
                    ("payload.body.urls[0]", ".link") => "payload.body.urls[0].link",
                    ("payload.body.urls[0]", ".target") => "payload.body.urls[0].target",
                    ("payload.request.body.urls[0]", ".url") => "payload.request.body.urls[0].url",
                    ("payload.request.body.urls[0]", ".href") => "payload.request.body.urls[0].href",
                    ("payload.request.body.urls[0]", ".uri") => "payload.request.body.urls[0].uri",
                    ("payload.request.body.urls[0]", ".link") => "payload.request.body.urls[0].link",
                    ("payload.request.body.urls[0]", ".target") => "payload.request.body.urls[0].target",
                    _ => "none",
                };
                if source_name != "none" {
                    return (url_candidate, source_name);
                }
            }
        }
    }
    for (key, source) in [("query", "query"), ("q", "q")] {
        let candidate = clean_text(
            request.get(key).and_then(Value::as_str).unwrap_or(""),
            2200,
        );
        if let Some(url_candidate) = fetch_url_candidate_from_text(&candidate) {
            return (url_candidate, source);
        }
    }
    for (key, source) in [
        ("message", "message"),
        ("text", "text"),
        ("input", "input"),
        ("prompt", "prompt"),
        ("question", "question"),
    ] {
        let candidate = clean_text(
            request.get(key).and_then(Value::as_str).unwrap_or(""),
            2200,
        );
        if let Some(url_candidate) = fetch_url_candidate_from_text(&candidate) {
            return (url_candidate, source);
        }
    }
    for (pointer, source) in [
        ("/request/message", "request.message"),
        ("/request/text", "request.text"),
        ("/request/input", "request.input"),
        ("/request/prompt", "request.prompt"),
        ("/request/question", "request.question"),
        ("/request/data/message", "request.data.message"),
        ("/request/data/text", "request.data.text"),
        ("/request/data/input", "request.data.input"),
        ("/request/data/prompt", "request.data.prompt"),
        ("/request/data/question", "request.data.question"),
        ("/request/body/data/message", "request.body.data.message"),
        ("/request/body/data/text", "request.body.data.text"),
        ("/request/body/data/input", "request.body.data.input"),
        ("/request/body/data/prompt", "request.body.data.prompt"),
        ("/request/body/data/question", "request.body.data.question"),
        ("/payload/message", "payload.message"),
        ("/payload/text", "payload.text"),
        ("/payload/input", "payload.input"),
        ("/payload/prompt", "payload.prompt"),
        ("/payload/question", "payload.question"),
        ("/payload/data/message", "payload.data.message"),
        ("/payload/data/text", "payload.data.text"),
        ("/payload/data/input", "payload.data.input"),
        ("/payload/data/prompt", "payload.data.prompt"),
        ("/payload/data/question", "payload.data.question"),
        ("/payload/body/data/message", "payload.body.data.message"),
        ("/payload/body/data/text", "payload.body.data.text"),
        ("/payload/body/data/input", "payload.body.data.input"),
        ("/payload/body/data/prompt", "payload.body.data.prompt"),
        ("/payload/body/data/question", "payload.body.data.question"),
        ("/payload/request/message", "payload.request.message"),
        ("/payload/request/text", "payload.request.text"),
        ("/payload/request/input", "payload.request.input"),
        ("/payload/request/prompt", "payload.request.prompt"),
        ("/payload/request/question", "payload.request.question"),
        ("/payload/request/data/message", "payload.request.data.message"),
        ("/payload/request/data/text", "payload.request.data.text"),
        ("/payload/request/data/input", "payload.request.data.input"),
        ("/payload/request/data/prompt", "payload.request.data.prompt"),
        ("/payload/request/data/question", "payload.request.data.question"),
        (
            "/payload/request/body/data/message",
            "payload.request.body.data.message",
        ),
        (
            "/payload/request/body/data/text",
            "payload.request.body.data.text",
        ),
        (
            "/payload/request/body/data/input",
            "payload.request.body.data.input",
        ),
        (
            "/payload/request/body/data/prompt",
            "payload.request.body.data.prompt",
        ),
        (
            "/payload/request/body/data/question",
            "payload.request.body.data.question",
        ),
    ] {
        let candidate = clean_text(
            request.pointer(pointer).and_then(Value::as_str).unwrap_or(""),
            2200,
        );
        if let Some(url_candidate) = fetch_url_candidate_from_text(&candidate) {
            return (url_candidate, source);
        }
    }
    (String::new(), "none")
}
