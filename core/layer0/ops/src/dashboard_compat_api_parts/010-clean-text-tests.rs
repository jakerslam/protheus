mod clean_text_runtime_access_tests {
    use super::runtime_access_denied_phrase;

    #[test]
    fn runtime_access_denied_phrase_catches_classic_denial() {
        assert!(runtime_access_denied_phrase(
            "I do not have access to runtime systems right now."
        ));
    }

    #[test]
    fn runtime_access_denied_phrase_catches_internal_meta_dump_pattern() {
        assert!(runtime_access_denied_phrase(
            "Yes—my apologies. I dumped internal memory metadata instead of actually answering your question. \
             That's a bug in my response generation. Which of the suggestions did you implement? \
             If you can tell me which lever you pulled, I can check what should I be looking for."
        ));
    }

    #[test]
    fn runtime_access_denied_phrase_ignores_normal_user_facing_status() {
        assert!(!runtime_access_denied_phrase(
            "Queue depth is low, workers are stable, and alerts are clear."
        ));
    }

    #[test]
    fn runtime_access_denied_phrase_catches_monitoring_tools_fallback() {
        assert!(runtime_access_denied_phrase(
            "I do not have access to runtime systems. Check your system monitoring tools."
        ));
    }

    #[test]
    fn runtime_access_denied_phrase_catches_workspace_only_introspection_dump() {
        assert!(runtime_access_denied_phrase(
            "I can only read what’s in your workspace files. I don’t have inherent introspection into my own codebase beyond what I can infer from runtime behavior."
        ));
    }

    #[test]
    fn runtime_access_denied_phrase_catches_no_web_access_variant() {
        assert!(runtime_access_denied_phrase(
            "No—still no web access in this environment."
        ));
    }
}

mod clean_text_swarm_intent_tests {
    use super::{
        infer_subagent_count_from_message, spawn_surface_denied_phrase, swarm_intent_requested,
    };

    #[test]
    fn swarm_intent_requested_detects_parallel_keywords() {
        assert!(swarm_intent_requested(
            "Please split this into parallel subagent lanes and run a swarm."
        ));
        assert!(swarm_intent_requested(
            "summon a swarm to parallelize this audit"
        ));
    }

    #[test]
    fn spawn_surface_denied_phrase_detects_capability_denial() {
        assert!(spawn_surface_denied_phrase(
            "I don’t currently see a command surface to spawn an arbitrary swarm of new agents."
        ));
    }

    #[test]
    fn infer_subagent_count_from_message_prefers_numeric_hint() {
        assert_eq!(
            infer_subagent_count_from_message("spawn 11 subagents now"),
            8
        );
        assert_eq!(
            infer_subagent_count_from_message("spawn 2 subagents now"),
            2
        );
    }
}

mod clean_text_memory_phrase_tests {
    use super::{
        internal_context_metadata_phrase, persistent_memory_denied_phrase,
        strip_internal_cache_control_markup, strip_internal_context_metadata_prefix,
    };

    #[test]
    fn persistent_memory_denied_phrase_catches_internal_metadata_summary() {
        assert!(persistent_memory_denied_phrase(
            "Persistent memory is enabled for this agent across 1 session(s) with 4 stored messages. Recalled context: favorite animal is octopus."
        ));
    }

    #[test]
    fn internal_context_metadata_phrase_catches_recalled_context_banner() {
        assert!(internal_context_metadata_phrase(
            "Persistent memory enabled across 2 sessions with 12 stored messages. Recalled context: alpha | beta | gamma"
        ));
    }

    #[test]
    fn strip_internal_context_metadata_prefix_drops_context_dump() {
        assert_eq!(
            strip_internal_context_metadata_prefix(
                "Persistent memory is enabled for this agent across 1 session(s) with 4 stored messages. Recalled context: alpha | beta | gamma"
            ),
            ""
        );
    }

    #[test]
    fn strip_internal_cache_control_markup_removes_cache_tags() {
        let cleaned = strip_internal_cache_control_markup(
            "I see marker <cache_control lane=\"autonomy\" stable_hash=\"abc123\" breakpoint=\"system_instructions\" /> and continue.",
        );
        assert!(!cleaned.contains("<cache_control"));
        assert!(!cleaned.contains("stable_hash="));
        assert!(cleaned.contains("I see marker"));
    }

    #[test]
    fn strip_internal_cache_control_markup_removes_plaintext_hash_line() {
        let cleaned = strip_internal_cache_control_markup(
            "cache telemetry: cache_control lane=\"autonomy\" stable_hash=\"6f4ed79ad92d4b86\"\nreal answer",
        );
        assert_eq!(cleaned, "real answer");
    }
}
