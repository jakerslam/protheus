fn search_query_and_source(request: &Value) -> (String, &'static str) {
    for (key, source) in [
        ("query", "query"),
        ("q", "q"),
        ("search_query", "search_query"),
        ("searchQuery", "searchQuery"),
        ("prompt", "prompt"),
        ("input", "input"),
        ("text", "text"),
        ("message", "message"),
        ("question", "question"),
    ] {
        let value = clean_text(request.get(key).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
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
        ("/request/input", "request.input"),
        ("/request/text", "request.text"),
        ("/request/message", "request.message"),
        ("/request/question", "request.question"),
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
        ("/payload/request/input", "payload.request.input"),
        ("/payload/request/text", "payload.request.text"),
        ("/payload/request/message", "payload.request.message"),
        ("/payload/request/question", "payload.request.question"),
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
        ("/payload/input", "payload.input"),
        ("/payload/text", "payload.text"),
        ("/payload/message", "payload.message"),
        ("/payload/question", "payload.question"),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (key, source) in [("queries", "queries[0]"), ("search_queries", "search_queries[0]")] {
        if let Some(value) = request
            .get(key)
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_str)
        {
            let cleaned = clean_text(value, 600);
            if !cleaned.trim().is_empty() {
                return (cleaned, source);
            }
        }
    }
    for (pointer, source) in [
        ("/request/queries/0", "request.queries[0]"),
        ("/request/search_queries/0", "request.search_queries[0]"),
        ("/payload/request/queries/0", "payload.request.queries[0]"),
        (
            "/payload/request/search_queries/0",
            "payload.request.search_queries[0]",
        ),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (pointer, source) in [
        ("/request/body/queries/0", "request.body.queries[0]"),
        (
            "/request/body/search_queries/0",
            "request.body.search_queries[0]",
        ),
        ("/payload/body/queries/0", "payload.body.queries[0]"),
        (
            "/payload/body/search_queries/0",
            "payload.body.search_queries[0]",
        ),
        (
            "/payload/request/body/queries/0",
            "payload.request.body.queries[0]",
        ),
        (
            "/payload/request/body/search_queries/0",
            "payload.request.body.search_queries[0]",
        ),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (key, source_prefix) in [("queries", "queries"), ("search_queries", "search_queries")] {
        if let Some(first_row) = request
            .get(key)
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
        {
            for (field, source) in [
                ("query", "query"),
                ("q", "q"),
                ("text", "text"),
                ("prompt", "prompt"),
                ("input", "input"),
                ("message", "message"),
            ] {
                let value = clean_text(first_row.get(field).and_then(Value::as_str).unwrap_or(""), 600);
                if !value.trim().is_empty() {
                    let source_name = if source_prefix == "queries" {
                        match source {
                            "query" => "queries[0].query",
                            "q" => "queries[0].q",
                            "text" => "queries[0].text",
                            "prompt" => "queries[0].prompt",
                            "input" => "queries[0].input",
                            "message" => "queries[0].message",
                            _ => "queries[0]",
                        }
                    } else {
                        match source {
                            "query" => "search_queries[0].query",
                            "q" => "search_queries[0].q",
                            "text" => "search_queries[0].text",
                            "prompt" => "search_queries[0].prompt",
                            "input" => "search_queries[0].input",
                            "message" => "search_queries[0].message",
                            _ => "search_queries[0]",
                        }
                    };
                    return (value, source_name);
                }
            }
        }
    }
    for (pointer, source) in [
        ("/payload/queries/0", "payload.queries[0]"),
        ("/payload/search_queries/0", "payload.search_queries[0]"),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    for (pointer, source_prefix) in [
        ("/payload/queries/0", "payload.queries[0]"),
        ("/payload/search_queries/0", "payload.search_queries[0]"),
    ] {
        for (field, source_suffix) in [
            ("query", ".query"),
            ("q", ".q"),
            ("text", ".text"),
            ("prompt", ".prompt"),
            ("input", ".input"),
            ("message", ".message"),
        ] {
            let pointer_with_field = format!("{}/{}", pointer, field);
            let value = clean_text(
                request.pointer(&pointer_with_field).and_then(Value::as_str).unwrap_or(""),
                600,
            );
            if !value.trim().is_empty() {
                let source_name = match (source_prefix, source_suffix) {
                    ("payload.queries[0]", ".query") => "payload.queries[0].query",
                    ("payload.queries[0]", ".q") => "payload.queries[0].q",
                    ("payload.queries[0]", ".text") => "payload.queries[0].text",
                    ("payload.queries[0]", ".prompt") => "payload.queries[0].prompt",
                    ("payload.queries[0]", ".input") => "payload.queries[0].input",
                    ("payload.queries[0]", ".message") => "payload.queries[0].message",
                    ("payload.search_queries[0]", ".query") => "payload.search_queries[0].query",
                    ("payload.search_queries[0]", ".q") => "payload.search_queries[0].q",
                    ("payload.search_queries[0]", ".text") => "payload.search_queries[0].text",
                    ("payload.search_queries[0]", ".prompt") => "payload.search_queries[0].prompt",
                    ("payload.search_queries[0]", ".input") => "payload.search_queries[0].input",
                    ("payload.search_queries[0]", ".message") => "payload.search_queries[0].message",
                    _ => "none",
                };
                if source_name != "none" {
                    return (value, source_name);
                }
            }
        }
    }
    for (pointer, source_prefix) in [
        ("/request/queries/0", "request.queries[0]"),
        ("/request/search_queries/0", "request.search_queries[0]"),
        ("/payload/request/queries/0", "payload.request.queries[0]"),
        (
            "/payload/request/search_queries/0",
            "payload.request.search_queries[0]",
        ),
    ] {
        for (field, source_suffix) in [
            ("query", ".query"),
            ("q", ".q"),
            ("text", ".text"),
            ("prompt", ".prompt"),
            ("input", ".input"),
            ("message", ".message"),
        ] {
            let pointer_with_field = format!("{}/{}", pointer, field);
            let value = clean_text(
                request.pointer(&pointer_with_field).and_then(Value::as_str).unwrap_or(""),
                600,
            );
            if !value.trim().is_empty() {
                let source_name = match (source_prefix, source_suffix) {
                    ("request.queries[0]", ".query") => "request.queries[0].query",
                    ("request.queries[0]", ".q") => "request.queries[0].q",
                    ("request.queries[0]", ".text") => "request.queries[0].text",
                    ("request.queries[0]", ".prompt") => "request.queries[0].prompt",
                    ("request.queries[0]", ".input") => "request.queries[0].input",
                    ("request.queries[0]", ".message") => "request.queries[0].message",
                    ("request.search_queries[0]", ".query") => "request.search_queries[0].query",
                    ("request.search_queries[0]", ".q") => "request.search_queries[0].q",
                    ("request.search_queries[0]", ".text") => "request.search_queries[0].text",
                    ("request.search_queries[0]", ".prompt") => "request.search_queries[0].prompt",
                    ("request.search_queries[0]", ".input") => "request.search_queries[0].input",
                    ("request.search_queries[0]", ".message") => "request.search_queries[0].message",
                    ("payload.request.queries[0]", ".query") => "payload.request.queries[0].query",
                    ("payload.request.queries[0]", ".q") => "payload.request.queries[0].q",
                    ("payload.request.queries[0]", ".text") => "payload.request.queries[0].text",
                    ("payload.request.queries[0]", ".prompt") => "payload.request.queries[0].prompt",
                    ("payload.request.queries[0]", ".input") => "payload.request.queries[0].input",
                    ("payload.request.queries[0]", ".message") => "payload.request.queries[0].message",
                    ("payload.request.search_queries[0]", ".query") => {
                        "payload.request.search_queries[0].query"
                    }
                    ("payload.request.search_queries[0]", ".q") => {
                        "payload.request.search_queries[0].q"
                    }
                    ("payload.request.search_queries[0]", ".text") => {
                        "payload.request.search_queries[0].text"
                    }
                    ("payload.request.search_queries[0]", ".prompt") => {
                        "payload.request.search_queries[0].prompt"
                    }
                    ("payload.request.search_queries[0]", ".input") => {
                        "payload.request.search_queries[0].input"
                    }
                    ("payload.request.search_queries[0]", ".message") => {
                        "payload.request.search_queries[0].message"
                    }
                    _ => "none",
                };
                if source_name != "none" {
                    return (value, source_name);
                }
            }
        }
    }
    for (pointer, source_prefix) in [
        ("/request/body/queries/0", "request.body.queries[0]"),
        (
            "/request/body/search_queries/0",
            "request.body.search_queries[0]",
        ),
        ("/payload/body/queries/0", "payload.body.queries[0]"),
        (
            "/payload/body/search_queries/0",
            "payload.body.search_queries[0]",
        ),
        (
            "/payload/request/body/queries/0",
            "payload.request.body.queries[0]",
        ),
        (
            "/payload/request/body/search_queries/0",
            "payload.request.body.search_queries[0]",
        ),
    ] {
        for (field, source_suffix) in [
            ("query", ".query"),
            ("q", ".q"),
            ("text", ".text"),
            ("prompt", ".prompt"),
            ("input", ".input"),
            ("message", ".message"),
            ("question", ".question"),
            ("search_query", ".search_query"),
            ("searchQuery", ".searchQuery"),
        ] {
            let pointer_with_field = format!("{}/{}", pointer, field);
            let value = clean_text(
                request.pointer(&pointer_with_field).and_then(Value::as_str).unwrap_or(""),
                600,
            );
            if !value.trim().is_empty() {
                let source_name = match (source_prefix, source_suffix) {
                    ("request.body.queries[0]", ".query") => "request.body.queries[0].query",
                    ("request.body.queries[0]", ".q") => "request.body.queries[0].q",
                    ("request.body.queries[0]", ".text") => "request.body.queries[0].text",
                    ("request.body.queries[0]", ".prompt") => "request.body.queries[0].prompt",
                    ("request.body.queries[0]", ".input") => "request.body.queries[0].input",
                    ("request.body.queries[0]", ".message") => "request.body.queries[0].message",
                    ("request.body.queries[0]", ".question") => "request.body.queries[0].question",
                    ("request.body.queries[0]", ".search_query") => {
                        "request.body.queries[0].search_query"
                    }
                    ("request.body.queries[0]", ".searchQuery") => {
                        "request.body.queries[0].searchQuery"
                    }
                    ("request.body.search_queries[0]", ".query") => {
                        "request.body.search_queries[0].query"
                    }
                    ("request.body.search_queries[0]", ".q") => "request.body.search_queries[0].q",
                    ("request.body.search_queries[0]", ".text") => {
                        "request.body.search_queries[0].text"
                    }
                    ("request.body.search_queries[0]", ".prompt") => {
                        "request.body.search_queries[0].prompt"
                    }
                    ("request.body.search_queries[0]", ".input") => {
                        "request.body.search_queries[0].input"
                    }
                    ("request.body.search_queries[0]", ".message") => {
                        "request.body.search_queries[0].message"
                    }
                    ("request.body.search_queries[0]", ".question") => {
                        "request.body.search_queries[0].question"
                    }
                    ("request.body.search_queries[0]", ".search_query") => {
                        "request.body.search_queries[0].search_query"
                    }
                    ("request.body.search_queries[0]", ".searchQuery") => {
                        "request.body.search_queries[0].searchQuery"
                    }
                    ("payload.body.queries[0]", ".query") => "payload.body.queries[0].query",
                    ("payload.body.queries[0]", ".q") => "payload.body.queries[0].q",
                    ("payload.body.queries[0]", ".text") => "payload.body.queries[0].text",
                    ("payload.body.queries[0]", ".prompt") => "payload.body.queries[0].prompt",
                    ("payload.body.queries[0]", ".input") => "payload.body.queries[0].input",
                    ("payload.body.queries[0]", ".message") => "payload.body.queries[0].message",
                    ("payload.body.queries[0]", ".question") => "payload.body.queries[0].question",
                    ("payload.body.queries[0]", ".search_query") => {
                        "payload.body.queries[0].search_query"
                    }
                    ("payload.body.queries[0]", ".searchQuery") => {
                        "payload.body.queries[0].searchQuery"
                    }
                    ("payload.body.search_queries[0]", ".query") => {
                        "payload.body.search_queries[0].query"
                    }
                    ("payload.body.search_queries[0]", ".q") => "payload.body.search_queries[0].q",
                    ("payload.body.search_queries[0]", ".text") => {
                        "payload.body.search_queries[0].text"
                    }
                    ("payload.body.search_queries[0]", ".prompt") => {
                        "payload.body.search_queries[0].prompt"
                    }
                    ("payload.body.search_queries[0]", ".input") => {
                        "payload.body.search_queries[0].input"
                    }
                    ("payload.body.search_queries[0]", ".message") => {
                        "payload.body.search_queries[0].message"
                    }
                    ("payload.body.search_queries[0]", ".question") => {
                        "payload.body.search_queries[0].question"
                    }
                    ("payload.body.search_queries[0]", ".search_query") => {
                        "payload.body.search_queries[0].search_query"
                    }
                    ("payload.body.search_queries[0]", ".searchQuery") => {
                        "payload.body.search_queries[0].searchQuery"
                    }
                    ("payload.request.body.queries[0]", ".query") => {
                        "payload.request.body.queries[0].query"
                    }
                    ("payload.request.body.queries[0]", ".q") => "payload.request.body.queries[0].q",
                    ("payload.request.body.queries[0]", ".text") => {
                        "payload.request.body.queries[0].text"
                    }
                    ("payload.request.body.queries[0]", ".prompt") => {
                        "payload.request.body.queries[0].prompt"
                    }
                    ("payload.request.body.queries[0]", ".input") => {
                        "payload.request.body.queries[0].input"
                    }
                    ("payload.request.body.queries[0]", ".message") => {
                        "payload.request.body.queries[0].message"
                    }
                    ("payload.request.body.queries[0]", ".question") => {
                        "payload.request.body.queries[0].question"
                    }
                    ("payload.request.body.queries[0]", ".search_query") => {
                        "payload.request.body.queries[0].search_query"
                    }
                    ("payload.request.body.queries[0]", ".searchQuery") => {
                        "payload.request.body.queries[0].searchQuery"
                    }
                    ("payload.request.body.search_queries[0]", ".query") => {
                        "payload.request.body.search_queries[0].query"
                    }
                    ("payload.request.body.search_queries[0]", ".q") => {
                        "payload.request.body.search_queries[0].q"
                    }
                    ("payload.request.body.search_queries[0]", ".text") => {
                        "payload.request.body.search_queries[0].text"
                    }
                    ("payload.request.body.search_queries[0]", ".prompt") => {
                        "payload.request.body.search_queries[0].prompt"
                    }
                    ("payload.request.body.search_queries[0]", ".input") => {
                        "payload.request.body.search_queries[0].input"
                    }
                    ("payload.request.body.search_queries[0]", ".message") => {
                        "payload.request.body.search_queries[0].message"
                    }
                    ("payload.request.body.search_queries[0]", ".question") => {
                        "payload.request.body.search_queries[0].question"
                    }
                    ("payload.request.body.search_queries[0]", ".search_query") => {
                        "payload.request.body.search_queries[0].search_query"
                    }
                    ("payload.request.body.search_queries[0]", ".searchQuery") => {
                        "payload.request.body.search_queries[0].searchQuery"
                    }
                    _ => "none",
                };
                if source_name != "none" {
                    return (value, source_name);
                }
            }
        }
    }
    for (pointer, source) in [
        ("/queries/0/question", "queries[0]"),
        ("/queries/0/search_query", "queries[0]"),
        ("/queries/0/searchQuery", "queries[0]"),
        ("/search_queries/0/question", "search_queries[0]"),
        ("/search_queries/0/search_query", "search_queries[0]"),
        ("/search_queries/0/searchQuery", "search_queries[0]"),
        ("/payload/queries/0/question", "payload.queries[0]"),
        ("/payload/queries/0/search_query", "payload.queries[0]"),
        ("/payload/queries/0/searchQuery", "payload.queries[0]"),
        ("/payload/search_queries/0/question", "payload.search_queries[0]"),
        (
            "/payload/search_queries/0/search_query",
            "payload.search_queries[0]",
        ),
        (
            "/payload/search_queries/0/searchQuery",
            "payload.search_queries[0]",
        ),
        ("/request/queries/0/question", "request.queries[0]"),
        ("/request/queries/0/search_query", "request.queries[0]"),
        ("/request/queries/0/searchQuery", "request.queries[0]"),
        (
            "/request/search_queries/0/question",
            "request.search_queries[0]",
        ),
        (
            "/request/search_queries/0/search_query",
            "request.search_queries[0]",
        ),
        (
            "/request/search_queries/0/searchQuery",
            "request.search_queries[0]",
        ),
        (
            "/payload/request/queries/0/question",
            "payload.request.queries[0]",
        ),
        (
            "/payload/request/queries/0/search_query",
            "payload.request.queries[0]",
        ),
        (
            "/payload/request/queries/0/searchQuery",
            "payload.request.queries[0]",
        ),
        (
            "/payload/request/search_queries/0/question",
            "payload.request.search_queries[0]",
        ),
        (
            "/payload/request/search_queries/0/search_query",
            "payload.request.search_queries[0]",
        ),
        (
            "/payload/request/search_queries/0/searchQuery",
            "payload.request.search_queries[0]",
        ),
    ] {
        let value = clean_text(request.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 600);
        if !value.trim().is_empty() {
            return (value, source);
        }
    }
    (String::new(), "none")
}
