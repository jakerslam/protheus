use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::tool_broker::BrokerCaller;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapability {
    pub tool_name: String,
    pub required_args: Vec<String>,
    pub allowed_callers: Vec<BrokerCaller>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapabilityProbe {
    pub tool_name: String,
    pub caller: BrokerCaller,
    pub available: bool,
    pub reason: String,
}

fn capability_matrix() -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::<String, Vec<String>>::new();
    out.insert("batch_query".to_string(), vec!["query".to_string()]);
    out.insert("file_read".to_string(), vec!["path".to_string()]);
    out.insert("file_read_many".to_string(), vec!["paths".to_string()]);
    out.insert("web_fetch".to_string(), vec!["url".to_string()]);
    out.insert("web_search".to_string(), vec!["query".to_string()]);
    out
}

pub fn all_capabilities_for_callers(
    allowed_tools: &std::collections::HashMap<BrokerCaller, std::collections::HashSet<String>>,
) -> Vec<ToolCapability> {
    let matrix = capability_matrix();
    let mut out = Vec::<ToolCapability>::new();
    for (tool_name, required_args) in matrix {
        let mut callers = allowed_tools
            .iter()
            .filter_map(|(caller, tools)| {
                if tools.contains(&tool_name) {
                    Some(*caller)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        callers.sort_by_key(|caller| match caller {
            BrokerCaller::Client => 0,
            BrokerCaller::Worker => 1,
            BrokerCaller::System => 2,
        });
        out.push(ToolCapability {
            tool_name,
            required_args,
            allowed_callers: callers,
        });
    }
    out
}

pub fn required_args_for(tool_name: &str) -> Vec<String> {
    capability_matrix()
        .get(tool_name)
        .cloned()
        .unwrap_or_default()
}
