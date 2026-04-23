// SPDX-License-Identifier: Apache-2.0
// SRS coverage: V11-ADAPTER-CHAOS-001

use infring_ops_core::framework_adapter_contract::execute_governed_workflow;
use serde_json::json;

#[test]
fn adapter_contract_fail_closed_chaos_matrix_is_enforced_for_all_production_frameworks() {
    let frameworks = ["langgraph", "crewai", "openai_agents", "mastra", "semantic_kernel"];
    let scenarios = [
        ("process_never_starts", "adapter_startup_timeout"),
        ("starts_then_hangs", "adapter_request_timeout"),
        ("invalid_schema_response", "adapter_invalid_schema"),
        ("response_too_large", "adapter_response_too_large"),
        ("repeated_flapping", "adapter_circuit_open"),
    ];

    for framework in frameworks {
        for (scenario, expected_error) in scenarios {
            let payload = json!({
                "task_id": format!("chaos_task_{framework}_{scenario}"),
                "trace_id": format!("chaos_trace_{framework}_{scenario}"),
                "tool_name": "web_search",
                "chaos_scenario": scenario,
                "tool_args": {
                    "query": format!("adapter chaos {scenario} for {framework}")
                }
            });
            let err = execute_governed_workflow(framework, payload.as_object().expect("obj"))
                .expect_err("chaos scenario should fail closed");
            assert!(
                err.contains(expected_error),
                "framework={framework} scenario={scenario} expected_error={expected_error} actual={err}"
            );
        }
    }
}

#[test]
fn adapter_contract_emits_contract_kit_metadata_for_all_production_frameworks() {
    let frameworks = ["langgraph", "crewai", "openai_agents", "mastra", "semantic_kernel"];
    for framework in frameworks {
        let payload = json!({
            "task_id": format!("baseline_task_{framework}"),
            "trace_id": format!("baseline_trace_{framework}"),
            "tool_name": "web_search",
            "tool_args": {
                "query": format!("adapter baseline for {framework}")
            },
            "raw_result": {
                "results": [
                    {
                        "source": format!("{framework}_adapter"),
                        "title": "adapter baseline result",
                        "summary": "baseline governed workflow path"
                    }
                ]
            }
        });
        let out = execute_governed_workflow(framework, payload.as_object().expect("obj"))
            .expect("baseline governed execution should succeed");
        assert_eq!(out.payload.get("ok").and_then(|row| row.as_bool()), Some(true));
        assert_eq!(
            out.payload
                .pointer("/adapter_contract_kit/contract_version")
                .and_then(|row| row.as_str()),
            Some("adapter_contract_kit_v1")
        );
        assert_eq!(
            out.payload
                .pointer("/adapter_contract_kit/fail_closed/enabled")
                .and_then(|row| row.as_bool()),
            Some(true)
        );
    }
}
