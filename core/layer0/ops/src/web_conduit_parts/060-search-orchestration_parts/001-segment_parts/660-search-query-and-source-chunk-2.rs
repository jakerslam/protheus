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
