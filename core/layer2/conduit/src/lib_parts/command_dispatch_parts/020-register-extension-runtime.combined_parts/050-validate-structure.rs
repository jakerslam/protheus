
fn validate_structure(command: &TsCommand) -> Option<String> {
    match command {
        TsCommand::StartAgent { agent_id } | TsCommand::StopAgent { agent_id } => {
            if agent_id.trim().is_empty() {
                return Some("agent_id_required".to_string());
            }
        }
        TsCommand::QueryReceiptChain { limit, .. } => {
            if let Some(value) = limit {
                if *value == 0 || *value > 1000 {
                    return Some("receipt_query_limit_out_of_range".to_string());
                }
            }
        }
        TsCommand::ApplyPolicyUpdate { patch_id, .. } => {
            if patch_id.trim().is_empty() {
                return Some("policy_patch_id_required".to_string());
            }
            if !patch_id.starts_with("constitution_safe/") {
                return Some("policy_update_must_be_constitution_safe".to_string());
            }
        }
        TsCommand::InstallExtension {
            extension_id,
            wasm_sha256,
            capabilities,
            plugin_type,
            wasm_component_path,
            ..
        } => {
            if extension_id.trim().is_empty() {
                return Some("extension_id_required".to_string());
            }
            if !is_valid_sha256(wasm_sha256) {
                return Some("extension_wasm_sha256_invalid".to_string());
            }
            if capabilities.is_empty() || capabilities.iter().any(|cap| cap.trim().is_empty()) {
                return Some("extension_capabilities_invalid".to_string());
            }
            if wasm_component_path
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .is_none()
            {
                return Some("extension_wasm_component_path_required".to_string());
            }
            if let Some(plugin_type) = plugin_type {
                if !is_valid_plugin_type(plugin_type.trim()) {
                    return Some("extension_plugin_type_invalid".to_string());
                }
            }
        }
        TsCommand::ListActiveAgents | TsCommand::GetSystemStatus => {}
    }
    None
}
