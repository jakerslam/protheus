fn resolve_core_shortcuts_family_ops2(cmd: &str, rest: &[String]) -> Option<Route> {
    match cmd {
        "eval" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "benchmark".to_string());
            let args = match sub.as_str() {
                "enable"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("neuralavb"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("enable-neuralavb".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                "experiment"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("loop"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("experiment-loop".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                "benchmark" => std::iter::once("benchmark".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                _ => {
                    if rest.is_empty() {
                        vec!["benchmark".to_string()]
                    } else {
                        rest.to_vec()
                    }
                }
            };
            Some(Route {
                script_rel: "core://eval-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "rl" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            let args = match sub.as_str() {
                "upgrade"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("infring-v2"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("rl-upgrade".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                "status" => std::iter::once("rl-status".to_string())
                    .chain(rest.iter().skip(1).cloned())
                    .collect::<Vec<_>>(),
                _ => {
                    if rest.is_empty() {
                        vec!["rl-status".to_string()]
                    } else {
                        rest.to_vec()
                    }
                }
            };
            Some(Route {
                script_rel: "core://eval-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "experiment"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("loop"))
                .unwrap_or(false) =>
        {
            let mut args = vec!["experiment-loop".to_string()];
            args.extend(rest.iter().skip(1).cloned());
            Some(Route {
                script_rel: "core://eval-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "model" => {
            let sub = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "status".to_string());
            if sub == "buy"
                && rest
                    .get(1)
                    .map(|v| v.trim().eq_ignore_ascii_case("credits"))
                    .unwrap_or(false)
            {
                let args = std::iter::once("buy-credits".to_string())
                    .chain(rest.iter().skip(2).cloned())
                    .collect::<Vec<_>>();
                return Some(Route {
                    script_rel: "core://intelligence-nexus".to_string(),
                    args,
                    forward_stdin: false,
                });
            }
            let args = match sub.as_str() {
                "optimize"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("minimax"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("optimize".to_string())
                        .chain(std::iter::once("--profile=minimax".to_string()))
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                "use"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("bitnet"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("bitnet-use".to_string())
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                "use"
                    if rest
                        .get(1)
                        .map(|v| v.trim().eq_ignore_ascii_case("cheap"))
                        .unwrap_or(false) =>
                {
                    std::iter::once("optimize".to_string())
                        .chain(std::iter::once("--profile=minimax".to_string()))
                        .chain(rest.iter().skip(2).cloned())
                        .collect::<Vec<_>>()
                }
                _ => {
                    if rest.is_empty() {
                        vec!["status".to_string()]
                    } else {
                        rest.to_vec()
                    }
                }
            };
            Some(Route {
                script_rel: "core://model-router".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "keys" => {
            let args = if rest.is_empty() {
                vec!["open".to_string()]
            } else {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "open".to_string());
                match sub.as_str() {
                    "add" => std::iter::once("add-key".to_string())
                        .chain(rest.iter().skip(1).cloned())
                        .collect::<Vec<_>>(),
                    "rotate" => std::iter::once("rotate-key".to_string())
                        .chain(rest.iter().skip(1).cloned())
                        .collect::<Vec<_>>(),
                    "revoke" | "remove" => std::iter::once("revoke-key".to_string())
                        .chain(rest.iter().skip(1).cloned())
                        .collect::<Vec<_>>(),
                    _ => rest.to_vec(),
                }
            };
            Some(Route {
                script_rel: "core://intelligence-nexus".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "graph" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://graph-toolkit".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "research" => {
            let mut firmware_mode = false;
            let mut firmware_input: Option<String> = None;
            let mut passthrough = Vec::<String>::new();
            let mut idx = 0usize;
            while idx < rest.len() {
                let token = rest[idx].trim();
                let lower = token.to_ascii_lowercase();
                if lower == "--firmware" {
                    firmware_mode = true;
                    if let Some(next) = rest.get(idx + 1) {
                        if !next.starts_with("--") {
                            firmware_input = Some(next.clone());
                            idx += 1;
                        }
                    }
                } else if lower.starts_with("--firmware=") {
                    firmware_mode = true;
                    let value = token.split_once('=').map(|(_, v)| v.trim()).unwrap_or("");
                    if !value.is_empty() {
                        firmware_input = Some(value.to_string());
                    }
                } else {
                    passthrough.push(rest[idx].clone());
                }
                idx += 1;
            }
            if firmware_mode {
                let mut args = vec![
                    "scan".to_string(),
                    "--dx-source=research-firmware".to_string(),
                ];
                let input = firmware_input
                    .or_else(|| {
                        passthrough
                            .iter()
                            .find(|arg| arg.starts_with("--input="))
                            .map(|arg| arg.trim_start_matches("--input=").to_string())
                    })
                    .or_else(|| {
                        passthrough
                            .iter()
                            .find(|arg| !arg.starts_with("--"))
                            .cloned()
                    });
                if let Some(path) = input {
                    args.push(format!("--input={path}"));
                }
                args.extend(passthrough.into_iter().filter(|arg| {
                    arg.starts_with("--")
                        && (arg.starts_with("--strict=")
                            || arg.starts_with("--format=")
                            || arg.starts_with("--rulepack=")
                            || arg.starts_with("--allow-raw-path=")
                            || arg.starts_with("--transport="))
                }));
                return Some(Route {
                    script_rel: "core://binary-vuln-plane".to_string(),
                    args,
                    forward_stdin: false,
                });
            }
            let mut args = if rest.is_empty() {
                vec!["status".to_string()]
            } else if rest
                .first()
                .map(|v| {
                    let x = v.trim().to_ascii_lowercase();
                    x.starts_with("--")
                        || x.starts_with("https://")
                        || x.starts_with("http://")
                        || x.starts_with("file://")
                })
                .unwrap_or(false)
            {
                std::iter::once("fetch".to_string())
                    .chain(rest.iter().cloned())
                    .collect::<Vec<_>>()
            } else {
                rest.to_vec()
            };
            let has_mode = args.iter().any(|arg| arg.starts_with("--mode="));
            let stealth_index = args.iter().position(|arg| {
                let value = arg.trim().to_ascii_lowercase();
                value == "--stealth"
                    || value == "--stealth=1"
                    || value == "--stealth=true"
                    || value == "--stealth=yes"
                    || value == "--stealth=on"
            });
            if let Some(idx) = stealth_index {
                args.remove(idx);
                if !has_mode {
                    args.push("--mode=stealth".to_string());
                }
            } else if args.first().map(|v| v.as_str()) == Some("fetch") && !has_mode {
                args.push("--mode=auto".to_string());
            }
            Some(Route {
                script_rel: "core://research-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "crawl" => {
            let mut args = vec!["goal-crawl".to_string()];
            let mut goal_tokens = Vec::<String>::new();
            let mut passthrough = Vec::<String>::new();
            for row in rest {
                if row.starts_with("--") {
                    passthrough.push(row.clone());
                } else {
                    goal_tokens.push(row.clone());
                }
            }
            if !goal_tokens.is_empty() {
                args.push(format!("--goal={}", goal_tokens.join(" ")));
            }
            args.extend(passthrough);
            Some(Route {
                script_rel: "core://research-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "map" => {
            let mut args = vec!["map-site".to_string()];
            if let Some(domain) = rest.first() {
                if !domain.starts_with("--") {
                    args.push(format!("--domain={}", domain.trim()));
                    args.extend(rest.iter().skip(1).cloned());
                } else {
                    args.extend(rest.iter().cloned());
                }
            }
            Some(Route {
                script_rel: "core://research-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "monitor" => {
            let mut args = vec!["monitor".to_string()];
            if let Some(url) = rest.first() {
                if !url.starts_with("--") {
                    args.push(format!("--url={}", url.trim()));
                    args.extend(rest.iter().skip(1).cloned());
                } else {
                    args.extend(rest.iter().cloned());
                }
            }
            Some(Route {
                script_rel: "core://research-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "assimilate" => {
            if rest.is_empty() {
                return None;
            }
            let target = rest
                .first()
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            let passthrough = rest.iter().skip(1).cloned().collect::<Vec<_>>();
            match target.as_str() {
                "scrape://scrapy-core" => {
                    let mut args = vec!["template-governance".to_string()];
                    args.extend(passthrough);
                    Some(Route {
                        script_rel: "core://research-plane".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "scrape://firecrawl-core" => {
                    let mut args = vec!["firecrawl-template-governance".to_string()];
                    args.extend(passthrough);
                    Some(Route {
                        script_rel: "core://research-plane".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "parse://doc2dict-core" => {
                    let mut args = vec!["template-governance".to_string()];
                    args.extend(passthrough);
                    Some(Route {
                        script_rel: "core://parse-plane".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "llamaindex" | "rag://llamaindex" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["register-connector".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://llamaindex-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "google-adk" | "workflow://google-adk" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["register-tool-manifest".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://google-adk-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "camel" | "workflow://camel" | "society://camel" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["import-dataset".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://camel-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "haystack" | "workflow://haystack" | "rag://haystack" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["register-pipeline".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://haystack-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "workflow_chain" | "workflow://workflow_chain" | "chains://workflow_chain" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["import-integration".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://workflow_chain-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "pydantic-ai" | "workflow://pydantic-ai" | "agents://pydantic-ai" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["register-agent".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://pydantic-ai-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "dspy" | "workflow://dspy" | "optimizer://dspy" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["import-integration".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://dspy-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "mastra" | "workflow://mastra" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["register-graph".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://mastra-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                "shannon" | "workflow://shannon" => {
                    let args = if passthrough.is_empty()
                        || passthrough
                            .first()
                            .map(|row| row.starts_with("--"))
                            .unwrap_or(false)
                    {
                        let mut args = vec!["assimilate-intake".to_string()];
                        args.extend(passthrough);
                        args
                    } else {
                        passthrough
                    };
                    Some(Route {
                        script_rel: "core://shannon-bridge".to_string(),
                        args,
                        forward_stdin: false,
                    })
                }
                _ => None,
            }
        }
        "parse" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                match sub.as_str() {
                    "doc" => {
                        let mut args = vec!["parse-doc".to_string()];
                        if let Some(path) = rest.get(1) {
                            if !path.starts_with("--") {
                                args.push(format!("--file={}", path.trim()));
                                args.extend(rest.iter().skip(2).cloned());
                            } else {
                                args.extend(rest.iter().skip(1).cloned());
                            }
                        } else {
                            args.extend(rest.iter().skip(1).cloned());
                        }
                        args
                    }
                    "visualize" | "viz" => {
                        let mut args = vec!["visualize".to_string()];
                        if let Some(path) = rest.get(1) {
                            if !path.starts_with("--") {
                                args.push(format!("--from-path={}", path.trim()));
                                args.extend(rest.iter().skip(2).cloned());
                            } else {
                                args.extend(rest.iter().skip(1).cloned());
                            }
                        } else {
                            args.extend(rest.iter().skip(1).cloned());
                        }
                        args
                    }
                    "export" => {
                        let mut args = vec!["export".to_string()];
                        if let Some(path) = rest.get(1) {
                            if !path.starts_with("--") {
                                args.push(format!("--from-path={}", path.trim()));
                                if let Some(out_path) = rest.get(2) {
                                    if !out_path.starts_with("--") {
                                        args.push(format!("--output-path={}", out_path.trim()));
                                        args.extend(rest.iter().skip(3).cloned());
                                    } else {
                                        args.extend(rest.iter().skip(2).cloned());
                                    }
                                } else {
                                    args.extend(rest.iter().skip(2).cloned());
                                }
                            } else {
                                args.extend(rest.iter().skip(1).cloned());
                            }
                        } else {
                            args.extend(rest.iter().skip(1).cloned());
                        }
                        args
                    }
                    _ => rest.to_vec(),
                }
            };
            Some(Route {
                script_rel: "core://parse-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "flow" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                match sub.as_str() {
                    "compile" | "build" => {
                        let mut args = vec!["compile".to_string()];
                        if let Some(path) = rest.get(1) {
                            if !path.starts_with("--") {
                                args.push(format!("--canvas-path={}", path.trim()));
                                args.extend(rest.iter().skip(2).cloned());
                            } else {
                                args.extend(rest.iter().skip(1).cloned());
                            }
                        } else {
                            args.extend(rest.iter().skip(1).cloned());
                        }
                        args
                    }
                    "debug" => {
                        let mut args = vec!["playground".to_string()];
                        if let Some(op) = rest.get(1) {
                            if !op.starts_with("--") {
                                args.push(format!("--op={}", op.trim()));
                                args.extend(rest.iter().skip(2).cloned());
                            } else {
                                args.extend(rest.iter().skip(1).cloned());
                            }
                        } else {
                            args.extend(rest.iter().skip(1).cloned());
                        }
                        args
                    }
                    "run" => {
                        let mut args = vec!["playground".to_string(), "--op=play".to_string()];
                        args.extend(rest.iter().skip(1).cloned());
                        args
                    }
                    "templates" => {
                        let mut args = vec!["template-governance".to_string()];
                        args.extend(rest.iter().skip(1).cloned());
                        args
                    }
                    "install" => {
                        let mut args = vec!["install".to_string()];
                        args.extend(rest.iter().skip(1).cloned());
                        args
                    }
                    "components" => {
                        let mut args = vec!["component-marketplace".to_string()];
                        args.extend(rest.iter().skip(1).cloned());
                        args
                    }
                    _ => rest.to_vec(),
                }
            };
            Some(Route {
                script_rel: "core://flow-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "mcp" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                let sub = rest
                    .first()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .unwrap_or_else(|| "status".to_string());
                match sub.as_str() {
                    "expose" => {
                        let mut args = vec!["expose".to_string()];
                        if let Some(agent) = rest.get(1) {
                            if !agent.starts_with("--") {
                                args.push(format!("--agent={}", agent.trim()));
                                args.extend(rest.iter().skip(2).cloned());
                            } else {
                                args.extend(rest.iter().skip(1).cloned());
                            }
                        } else {
                            args.extend(rest.iter().skip(1).cloned());
                        }
                        args
                    }
                    _ => rest.to_vec(),
                }
            };
            Some(Route {
                script_rel: "core://mcp-plane".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "blobs" | "blob" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://binary-blob-runtime".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "directives" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://directive-kernel".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "prime"
            if rest
                .first()
                .map(|v| v.trim().eq_ignore_ascii_case("sign"))
                .unwrap_or(false) =>
        {
            let args = std::iter::once("prime-sign".to_string())
                .chain(rest.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(Route {
                script_rel: "core://directive-kernel".to_string(),
                args,
                forward_stdin: false,
            })
        }
        "organism" => {
            let args = if rest.is_empty() {
                vec!["status".to_string()]
            } else {
                rest.to_vec()
            };
            Some(Route {
                script_rel: "core://organism-layer".to_string(),
                args,
                forward_stdin: false,
            })
        }
        _ => resolve_core_shortcuts_family_ops3(cmd, rest),
    }
}
