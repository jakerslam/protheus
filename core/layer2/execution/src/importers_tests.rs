// SPDX-License-Identifier: Apache-2.0
use super::{
    parse_simple_yaml_text, run_importer_generic_json_json, run_importer_generic_yaml_json,
    run_importer_infring_json, run_importer_workflow_graph_json,
};
use serde_json::{json, Value};

#[test]
fn importer_generic_json_maps_arrays_and_objects() {
    let payload = json!({
        "prompts": [{"id": "p1"}, {"id": "p2"}],
        "settings": {"retries": 3}
    });
    let out = run_importer_generic_json_json(&payload.to_string())
        .expect("importer_generic_json_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["payload"]["source_item_count"], 3);
    assert_eq!(parsed["payload"]["mapped_item_count"], 3);

    let records = parsed["payload"]["entities"]["records"]
        .as_array()
        .expect("records array");
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["id"], "prompts_1");
    assert_eq!(records[1]["id"], "prompts_2");
    assert_eq!(records[2]["id"], "settings");
}

#[test]
fn importer_generic_json_empty_key_falls_back_to_record_prefix() {
    let payload = json!({
        "": [{"id": "x"}]
    });
    let out = run_importer_generic_json_json(&payload.to_string())
        .expect("importer_generic_json_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    let records = parsed["payload"]["entities"]["records"]
        .as_array()
        .expect("records array");
    assert_eq!(records[0]["id"], "record_1");
}

#[test]
fn importer_generic_json_non_object_payload_is_empty() {
    let out = run_importer_generic_json_json("[]")
        .expect("importer_generic_json_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["payload"]["source_item_count"], 0);
    assert_eq!(parsed["payload"]["mapped_item_count"], 0);
}

#[test]
fn parse_simple_yaml_text_maps_scalar_values() {
    let parsed = parse_simple_yaml_text(
        r#"
            # comment
            enabled: true
            retries: 3
            threshold: 2.5
            name: "alpha"
            "#,
    );
    assert_eq!(parsed["enabled"], true);
    assert_eq!(parsed["retries"], 3);
    assert_eq!(parsed["threshold"], 2.5);
    assert_eq!(parsed["name"], "alpha");
}

#[test]
fn importer_generic_yaml_string_payload_routes_to_generic_json_mapping() {
    let payload = "enabled: true\nretries: 3\n";
    let out =
        run_importer_generic_yaml_json(&serde_json::to_string(payload).expect("serialize payload"))
            .expect("importer_generic_yaml_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["payload"]["source_item_count"], 2);
    assert_eq!(parsed["payload"]["mapped_item_count"], 2);
}

#[test]
fn importer_generic_yaml_object_payload_passthrough() {
    let payload = json!({
        "prompts": [{"id": "p1"}],
        "settings": {"mode": "safe"}
    });
    let out = run_importer_generic_yaml_json(&payload.to_string())
        .expect("importer_generic_yaml_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["payload"]["source_item_count"], 2);
    assert_eq!(parsed["payload"]["mapped_item_count"], 2);
}

#[test]
fn importer_infring_maps_named_rows() {
    let payload = json!({
        "agents": [{"name": "Planner"}],
        "tasks": [{"id": "task_alpha"}],
        "workflows": [{"name": "PrimaryFlow"}],
        "tools": [{"name": "Search"}]
    });
    let out = run_importer_infring_json(&payload.to_string())
        .expect("importer_infring_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["payload"]["source_item_count"], 4);
    assert_eq!(parsed["payload"]["mapped_item_count"], 4);
    assert_eq!(parsed["payload"]["entities"]["agents"][0]["id"], "planner");
    assert_eq!(
        parsed["payload"]["entities"]["tasks"][0]["id"],
        "task_alpha"
    );
    assert_eq!(
        parsed["payload"]["entities"]["workflows"][0]["id"],
        "primaryflow"
    );
    assert_eq!(parsed["payload"]["entities"]["tools"][0]["id"], "search");
}

#[test]
fn importer_infring_non_object_payload_is_empty() {
    let out = run_importer_infring_json("[]").expect("importer_infring_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["payload"]["source_item_count"], 0);
    assert_eq!(parsed["payload"]["mapped_item_count"], 0);
}

#[test]
fn importer_workflow_graph_maps_nodes_and_edges() {
    let payload = json!({
        "nodes": [{"id": "a"}, {"id": "b"}],
        "edges": [{"from": "a", "to": "b"}]
    });
    let out = run_importer_workflow_graph_json(&payload.to_string())
        .expect("importer_workflow_graph_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["payload"]["source_item_count"], 3);
    assert_eq!(parsed["payload"]["mapped_item_count"], 3);
    assert_eq!(parsed["payload"]["entities"]["workflows"][0]["id"], "a");
    assert_eq!(
        parsed["payload"]["entities"]["workflows"][0]["edges_out"],
        1
    );
    assert_eq!(parsed["payload"]["entities"]["records"][0]["id"], "edge_1");
}

#[test]
fn importer_workflow_graph_non_object_payload_is_empty() {
    let out = run_importer_workflow_graph_json("[]")
        .expect("importer_workflow_graph_json should return output");
    let parsed: Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["payload"]["source_item_count"], 0);
    assert_eq!(parsed["payload"]["mapped_item_count"], 0);
}
