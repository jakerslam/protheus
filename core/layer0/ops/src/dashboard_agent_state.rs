// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0

#[path = "dashboard_agent_state_controls.rs"]
mod dashboard_agent_state_controls;
#[path = "dashboard_agent_state_registry.rs"]
mod dashboard_agent_state_registry;
#[path = "dashboard_agent_state_sessions.rs"]
mod dashboard_agent_state_sessions;

pub use dashboard_agent_state_controls::{
    create_session, delete_session, memory_kv_delete, memory_kv_get, memory_kv_pairs,
    memory_kv_semantic_query, memory_kv_set, switch_session,
};
pub use dashboard_agent_state_registry::{
    archive_agent, archived_agent_ids, delete_all_terminated, delete_terminated,
    enforce_expired_contracts, merge_profiles_into_collab, revive_agent, terminated_entries,
    unarchive_agent, upsert_contract, upsert_profile,
};
pub use dashboard_agent_state_sessions::{
    append_turn, load_session, seed_intro_message, session_summaries, suggestions,
};
