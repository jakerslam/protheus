
const TIER1_ROUTE_CONTRACTS: &[Tier1RouteContract] = &[
    Tier1RouteContract {
        cmd: "help",
        rest: &[],
        expected_script: "core://command-list",
    },
    Tier1RouteContract {
        cmd: "list",
        rest: &[],
        expected_script: "core://command-list",
    },
    Tier1RouteContract {
        cmd: "completion",
        rest: &[],
        expected_script: "core://completion",
    },
    Tier1RouteContract {
        cmd: "repl",
        rest: &[],
        expected_script: "core://repl",
    },
    Tier1RouteContract {
        cmd: "gateway",
        rest: &["status"],
        expected_script: "core://daemon-control",
    },
    Tier1RouteContract {
        cmd: "recover",
        rest: &[],
        expected_script: "core://daemon-control",
    },
    Tier1RouteContract {
        cmd: "dashboard",
        rest: &["status"],
        expected_script: "core://daemon-control",
    },
    Tier1RouteContract {
        cmd: "doctor",
        rest: &[],
        expected_script: "core://install-doctor",
    },
    Tier1RouteContract {
        cmd: "verify-install",
        rest: &[],
        expected_script: "core://install-doctor",
    },
    Tier1RouteContract {
        cmd: "version",
        rest: &[],
        expected_script: "core://version-cli",
    },
    Tier1RouteContract {
        cmd: "update",
        rest: &[],
        expected_script: "core://version-cli",
    },
];

const TIER1_RUNTIME_ENTRYPOINTS: &[&str] = &[
    "client/runtime/systems/ops/infringd.ts",
    "client/runtime/systems/ops/infring_status_dashboard.ts",
    "client/runtime/systems/ops/infring_unknown_guard.ts",
];

const COMMAND_ALIASES: &[CommandAlias] = &[
    CommandAlias {
        alias: "--help",
        canonical: "help",
    },
    CommandAlias {
        alias: "-h",
        canonical: "help",
    },
    CommandAlias {
        alias: "--version",
        canonical: "version",
    },
    CommandAlias {
        alias: "-v",
        canonical: "version",
    },
    CommandAlias {
        alias: "boot",
        canonical: "gateway",
    },
    CommandAlias {
        alias: "start",
        canonical: "gateway",
    },
    CommandAlias {
        alias: "stop",
        canonical: "gateway",
    },
    CommandAlias {
        alias: "restart",
        canonical: "gateway",
    },
    CommandAlias {
        alias: "repair",
        canonical: "recover",
    },
    CommandAlias {
        alias: "kairos",
        canonical: "proactive_daemon",
    },
];

impl CommandItem {
    pub fn synopsis(self) -> &'static str {
        self.synopsis
    }

    pub fn expected_script(self) -> &'static str {
        self.script_rel
    }

    pub fn handler_kind(self) -> CommandHandlerKind {
        self.handler
    }

    pub fn tier_kind(self) -> CommandTier {
        self.tier
    }

    pub fn availability_flag(self) -> &'static str {
        match self.handler {
            CommandHandlerKind::CoreDomain => "core_native",
            CommandHandlerKind::RuntimeScript => "runtime_wrapper_required",
        }
    }
}

pub fn command_registry() -> &'static [CommandItem] {
    COMMANDS
}

pub fn command_aliases() -> &'static [CommandAlias] {
    COMMAND_ALIASES
}

pub fn tier1_route_contracts() -> &'static [Tier1RouteContract] {
    TIER1_ROUTE_CONTRACTS
}

pub fn tier1_runtime_entrypoints() -> Vec<&'static str> {
    let mut out = TIER1_RUNTIME_ENTRYPOINTS.to_vec();
    out.sort_unstable();
    out
}

pub fn tier1_command_synopses() -> Vec<&'static str> {
    COMMANDS
        .iter()
        .filter(|row| matches!(row.tier, CommandTier::Tier1))
        .map(|row| row.synopsis)
        .collect::<Vec<_>>()
}

fn command_token_from_synopsis(synopsis: &str) -> String {
    synopsis
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn known_command_tokens() -> BTreeSet<String> {
    COMMANDS
        .iter()
        .map(|row| command_token_from_synopsis(row.synopsis))
        .filter(|token| !token.is_empty())
        .collect::<BTreeSet<_>>()
}

pub fn canonical_command_name(raw: &str) -> Option<String> {
    let token = raw.trim().to_ascii_lowercase();
    if token.is_empty() {
        return None;
    }
    let known = known_command_tokens();
    if known.contains(&token) {
        return Some(token);
    }
    let mut candidates = COMMAND_ALIASES
        .iter()
        .filter(|row| row.alias.eq_ignore_ascii_case(token.as_str()))
        .map(|row| row.canonical.to_ascii_lowercase())
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    candidates.dedup();
    if candidates.len() == 1 && known.contains(&candidates[0]) {
        return candidates.into_iter().next();
    }
    None
}

pub fn command_registry_item(raw: &str) -> Option<CommandItem> {
    let canonical = canonical_command_name(raw)?;
    COMMANDS
        .iter()
        .copied()
        .find(|row| command_token_from_synopsis(row.synopsis) == canonical)
}

pub fn command_registry_integrity() -> Value {
    let mut seen = BTreeSet::<String>::new();
    let mut duplicates = Vec::<String>::new();
    let known = known_command_tokens();
    let mut alias_collisions = BTreeMap::<String, BTreeSet<String>>::new();
    let mut alias_unknown_targets = Vec::<String>::new();
    let mut tier1 = 0usize;
    let mut experimental = 0usize;
    for row in COMMANDS {
        let key = row.synopsis.to_ascii_lowercase();
        if !seen.insert(key) {
            duplicates.push(row.synopsis.to_string());
        }
        match row.tier {
            CommandTier::Tier1 => tier1 += 1,
            CommandTier::Experimental => experimental += 1,
        }
    }
    for alias in COMMAND_ALIASES {
        let key = alias.alias.to_ascii_lowercase();
        alias_collisions
            .entry(key)
            .or_default()
            .insert(alias.canonical.to_ascii_lowercase());
        if !known.contains(&alias.canonical.to_ascii_lowercase()) {
            alias_unknown_targets.push(format!("{}->{}", alias.alias, alias.canonical));
        }
    }
    let alias_collisions = alias_collisions
        .into_iter()
        .filter_map(|(alias, targets)| {
            if targets.len() > 1 {
                Some(format!(
                    "{}->{}",
                    alias,
                    targets.into_iter().collect::<Vec<_>>().join("|")
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    duplicates.sort_unstable();
    alias_unknown_targets.sort_unstable();
    json!({
        "ok": duplicates.is_empty() && alias_collisions.is_empty() && alias_unknown_targets.is_empty(),
        "total": COMMANDS.len(),
        "tier1": tier1,
        "experimental": experimental,
        "duplicates": duplicates,
        "alias_collisions": alias_collisions,
        "alias_unknown_targets": alias_unknown_targets,
        "tier1_runtime_entrypoints": tier1_runtime_entrypoints(),
    })
}
