
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("demo");
    if command == "importer-web-tooling-signal" {
        match load_payload(&args[1..]) {
            Ok(payload) => match run_importer_web_tooling_signal_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        }
        return;
    }

    match command {
        "run" => match load_yaml(&args[1..]) {
            Ok(yaml) => {
                println!("{}", run_workflow_json(&yaml));
            }
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "decompose" => match load_payload(&args[1..]) {
            Ok(payload) => match decompose_goal_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "compose" => match load_payload(&args[1..]) {
            Ok(payload) => match compose_micro_tasks_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "task-summary" => match load_payload(&args[1..]) {
            Ok(payload) => match summarize_tasks_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "dispatch-summary" => match load_payload(&args[1..]) {
            Ok(payload) => match summarize_dispatch_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "queue-rows" => match load_payload(&args[1..]) {
            Ok(payload) => match queue_rows_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "dispatch-rows" => match load_payload(&args[1..]) {
            Ok(payload) => match dispatch_rows_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "directive-gate" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_directive_gate_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "route-primitives" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_route_primitives_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "route-match" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_route_match_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "route-reflex-match" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_route_reflex_match_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "route-complexity" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_route_complexity_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "route-evaluate" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_route_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "route-decision" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_route_decision_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "route-habit-readiness" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_route_habit_readiness_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "initiative-score" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_importance_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "initiative-action" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_initiative_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "attention-priority" => match load_payload(&args[1..]) {
            Ok(payload) => match prioritize_attention_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "heroic-gate" => match load_payload(&args[1..]) {
            Ok(payload) => match evaluate_heroic_gate_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "apply-governance" => match load_payload(&args[1..]) {
            Ok(payload) => match apply_governance_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "sprint-contract" => match load_payload(&args[1..]) {
            Ok(payload) => match run_sprint_contract_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "autoscale" => match load_payload(&args[1..]) {
            Ok(payload) => match run_autoscale_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "inversion" => match load_payload(&args[1..]) {
            Ok(payload) => match run_inversion_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "importer-generic-json" => match load_payload(&args[1..]) {
            Ok(payload) => match run_importer_generic_json_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "importer-generic-yaml" => match load_payload(&args[1..]) {
            Ok(payload) => match run_importer_generic_yaml_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "importer-infring" => match load_payload(&args[1..]) {
            Ok(payload) => match run_importer_infring_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "importer-workflow-graph" => match load_payload(&args[1..]) {
            Ok(payload) => match run_importer_workflow_graph_json(&payload) {
                Ok(out) => println!("{}", out),
                Err(err) => {
                    let payload = serde_json::json!({ "ok": false, "error": err });
                    eprintln!("{}", payload);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "demo" => {
            let demo = serde_json::json!({
                "workflow_id": "execution_demo",
                "deterministic_seed": "demo_seed",
                "pause_after_step": "score",
                "steps": [
                    {
                        "id": "collect",
                        "kind": "task",
                        "action": "collect_data",
                        "command": "collect --source=eyes"
                    },
                    {
                        "id": "score",
                        "kind": "task",
                        "action": "score",
                        "command": "score --strategy=deterministic"
                    },
                    {
                        "id": "ship",
                        "kind": "task",
                        "action": "ship",
                        "command": "ship --mode=canary"
                    }
                ]
            })
            .to_string();
            let receipt = run_workflow(&demo);
            println!(
                "{}",
                serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
            );
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
