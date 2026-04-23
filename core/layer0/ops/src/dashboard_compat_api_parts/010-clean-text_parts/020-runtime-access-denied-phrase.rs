
fn runtime_access_denied_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let normalized = lowered
        .replace('’', "'")
        .replace('`', "'")
        .replace('\u{201c}', "\"")
        .replace('\u{201d}', "\"");
    let internal_meta_dump = normalized.contains("internal memory metadata")
        || normalized.contains("instead of actually answering your question")
        || normalized.contains("bug in my response generation")
        || normalized.contains("my response generation")
        || normalized.contains("which of the suggestions did you implement")
        || normalized.contains("if you can tell me which lever you pulled")
        || normalized.contains("what should i be looking for");
    let workspace_only_capability_dump = normalized
        .contains("i can only read what's in your workspace files")
        || normalized.contains("i can only read what is in your workspace files")
        || normalized.contains("i don't have inherent introspection")
        || normalized.contains("i do not have inherent introspection")
        || normalized.contains("beyond what i can infer from runtime behavior")
        || normalized.contains("this particular instance appears under-provisioned")
        || normalized.contains("heavily sandboxed")
        || normalized.contains("missing basic fetch capabilities");
    normalized.contains("don't have access")
        || normalized.contains("do not have access")
        || normalized.contains("cannot access")
        || normalized.contains("no web access")
        || normalized.contains("no internet access")
        || normalized.contains("text-based ai assistant without system monitoring capabilities")
        || normalized.contains("without system monitoring")
        || normalized.contains("text-based ai assistant")
        || normalized.contains("cannot directly interface")
        || normalized.contains("cannot execute the infring-ops commands")
        || normalized.contains("check your system monitoring tools")
        || normalized.contains("no access to")
        || workspace_only_capability_dump
        || internal_meta_dump
}

fn internal_context_metadata_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let has_recalled_context = lowered.contains("recalled context:");
    let has_persistent_memory = lowered.contains("persistent memory");
    let has_stored_messages = lowered.contains("stored messages");
    let has_session_count = lowered.contains("session(s)") || lowered.contains(" sessions");
    (has_recalled_context && (has_persistent_memory || has_stored_messages || has_session_count))
        || (has_persistent_memory && has_stored_messages)
}

fn runtime_prompt_dump_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("you are the currently selected infring agent instance")
        || lowered
            .contains("hardcoded agent workflow: you are writing the final assistant response")
}

fn strip_internal_context_metadata_prefix(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if runtime_prompt_dump_phrase(trimmed) {
        return String::new();
    }
    let lowered = trimmed.to_ascii_lowercase();
    let Some(marker_idx) = lowered.find("recalled context:") else {
        return trimmed.to_string();
    };
    let prefix = &lowered[..marker_idx];
    let internal_prefix = prefix.contains("persistent memory")
        || prefix.contains("stored messages")
        || prefix.contains("session(s)")
        || prefix.contains(" sessions");
    if !internal_prefix {
        return trimmed.to_string();
    }
    let suffix = trimmed
        .split_once("Recalled context:")
        .map(|(_, tail)| tail)
        .or_else(|| {
            trimmed
                .split_once("recalled context:")
                .map(|(_, tail)| tail)
        })
        .or_else(|| {
            trimmed
                .split_once("RECALLED CONTEXT:")
                .map(|(_, tail)| tail)
        })
        .unwrap_or("")
        .trim();
    if suffix.is_empty() {
        return String::new();
    }
    if let Some((_, tail)) = suffix.split_once("\n\n") {
        let cleaned = tail.trim();
        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }
    if let Some((_, tail)) = suffix.split_once("Final answer:") {
        let cleaned = tail.trim();
        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }
    if let Some((_, tail)) = suffix.split_once("Answer:") {
        let cleaned = tail.trim();
        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }
    String::new()
}

fn strip_internal_cache_control_markup(text: &str) -> String {
    let mut cleaned = clean_chat_text(text, 64_000);
    loop {
        let lowered = cleaned.to_ascii_lowercase();
        let Some(start) = lowered.find("<cache_control") else {
            break;
        };
        let tail = &lowered[start..];
        let end_rel = tail
            .find("/>")
            .map(|idx| idx + 2)
            .or_else(|| {
                tail.find("</cache_control>")
                    .map(|idx| idx + "</cache_control>".len())
            })
            .or_else(|| tail.find('>').map(|idx| idx + 1))
            .unwrap_or(tail.len());
        let end = start.saturating_add(end_rel).min(cleaned.len());
        if end <= start {
            break;
        }
        cleaned.replace_range(start..end, "");
    }
    cleaned
        .lines()
        .filter(|line| {
            let lowered = line.to_ascii_lowercase();
            !(lowered.contains("stable_hash=")
                && (lowered.contains("cache_control") || lowered.contains("cache control")))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn persistent_memory_denied_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let conduit_gated_memory_denial = (lowered.contains("memory conduit")
        || lowered.contains("cockpit block")
        || lowered.contains("active memory context"))
        && (lowered.contains("current message")
            || lowered.contains("do not retain information")
            || lowered.contains("between exchanges")
            || lowered.contains("create a memory context"));
    lowered.contains("don't have persistent memory")
        || lowered.contains("do not have persistent memory")
        || lowered.contains("cannot recall our conversation")
        || lowered.contains("cannot recall the specific content")
        || lowered.contains("cannot recall previous conversation")
        || lowered.contains("cannot recall previous sessions")
        || lowered.contains("do not retain memory")
        || lowered.contains("don't retain memory")
        || lowered.contains("between sessions")
        || lowered.contains("session is stateless")
        || lowered.contains("each session is stateless")
        || lowered.contains("without persistent memory")
        || lowered.contains("within this session")
        || lowered.contains("do not retain information between exchanges")
        || lowered.contains("don't detect an active memory context")
        || lowered.contains("do not detect an active memory context")
        || lowered.contains("within active runtime scope")
        || lowered.contains("unless you explicitly use a memory conduit")
        || lowered.contains("persistent memory is enabled for this agent across")
        || lowered.contains("recalled context:")
        || internal_context_metadata_phrase(text)
        || conduit_gated_memory_denial
}

fn memory_recall_requested(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("remember")
        || lowered.contains("recall")
        || lowered.contains("last week")
        || lowered.contains("earlier")
        || lowered.contains("previous session")
        || lowered.contains("what did i ask")
}

fn runtime_probe_requested(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    (lowered.contains("queue depth")
        || lowered.contains("active workers")
        || lowered.contains("live signals")
        || lowered.contains("cockpit blocks")
        || lowered.contains("conduit signals")
        || lowered.contains("memory context")
        || lowered.contains("runtime sync")
        || lowered.contains("what changed")
        || lowered.contains("attention queue"))
        && (lowered.contains("runtime")
            || lowered.contains("status")
            || lowered.contains("sync")
            || lowered.contains("report")
            || lowered.contains("now"))
}

fn swarm_intent_requested(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("swarm")
        || lowered.contains("summon swarm")
        || lowered.contains("summon a swarm")
        || lowered.contains("subagent")
        || lowered.contains("sub-agent")
        || lowered.contains("descendant agent")
        || lowered.contains("parallel")
        || lowered.contains("split into")
        || lowered.contains("spawn agent")
        || lowered.contains("spawn workers")
        || lowered.contains("spin up agents")
}

fn spawn_surface_denied_phrase(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    (lowered.contains("command surface") && lowered.contains("spawn"))
        || (lowered.contains("don't currently see") && lowered.contains("spawn"))
        || (lowered.contains("do not currently see") && lowered.contains("spawn"))
        || (lowered.contains("requires") && lowered.contains("activation path"))
        || (lowered.contains("might require") && lowered.contains("runtime instances"))
        || (lowered.contains("don't have") && lowered.contains("swarm"))
        || (lowered.contains("do not have") && lowered.contains("swarm"))
}
