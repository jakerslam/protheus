// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

#[derive(Clone, Copy)]
pub enum CommandTier {
    Tier1,
    Experimental,
}

impl CommandTier {
    fn as_str(self) -> &'static str {
        match self {
            CommandTier::Tier1 => "tier1",
            CommandTier::Experimental => "experimental",
        }
    }
}

#[derive(Clone, Copy)]
pub enum CommandHandlerKind {
    CoreDomain,
    RuntimeScript,
}

impl CommandHandlerKind {
    fn as_str(self) -> &'static str {
        match self {
            CommandHandlerKind::CoreDomain => "core_domain",
            CommandHandlerKind::RuntimeScript => "runtime_script",
        }
    }
}

#[derive(Clone, Copy)]
pub struct CommandItem {
    synopsis: &'static str,
    desc: &'static str,
    tier: CommandTier,
    handler: CommandHandlerKind,
    script_rel: &'static str,
    read_only: bool,
    unsafe_surface: bool,
}

#[derive(Clone, Copy)]
pub struct CommandAlias {
    pub alias: &'static str,
    pub canonical: &'static str,
}

#[derive(Clone, Copy)]
pub struct Tier1RouteContract {
    pub cmd: &'static str,
    pub rest: &'static [&'static str],
    pub expected_script: &'static str,
}

const COMMANDS: &[CommandItem] = &[
    CommandItem {
        synopsis: "help",
        desc: "Show CLI help and command list.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://command-list",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "list",
        desc: "Show compact command list.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://command-list",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "gateway [start|stop|restart|status] [--gateway-persist=1|0]",
        desc:
            "One-command runtime gateway (boots rich dashboard + persistent supervisor by default).",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://daemon-control",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis:
            "recover [--dashboard-host=127.0.0.1] [--dashboard-port=4173] [--wait-max=90]",
        desc: "Run deterministic runtime recovery (stop/restart/revalidate) in one command.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://daemon-control",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "status",
        desc: "Show daemon/control-plane status.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://daemon-control",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "dashboard",
        desc: "Launch unified dashboard web UI (localhost).",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://daemon-control",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "dream",
        desc: "Run autonomy dream/consolidation lane.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://autonomy-controller",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "compact",
        desc: "Run autonomy compact lane.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://autonomy-controller",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "proactive_daemon [status|cycle|pause|resume]",
        desc: "Run bounded KAIROS proactive daemon controls.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://autonomy-controller",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "speculate [run|status|merge|reject]",
        desc: "Run autonomy speculation lane.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://autonomy-controller",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "doctor",
        desc: "Run install/runtime diagnostics.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://install-doctor",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "verify-install",
        desc: "Run strict install/runtime verification checks.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://install-doctor",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "stack <create|list|archive|tail-merge|tail-promote|render|batch-class|scheduler-check|node-spike|contract-verify|taste-tune|partial-merge|hybrid-retrieve|status|policy>",
        desc: "Manage context stacks for cacheable memory grouping.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://context-stacks",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "workspace-search <search|list|status> [--workspace=<path>] [--q=<query>] [--type=file|folder]",
        desc: "Search workspace files/folders with ripgrep-backed fuzzy ranking.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://workspace-file-search",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "alpha-check",
        desc: "Run alpha readiness checks.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://alpha-readiness",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1] [--allow-local-simulation=1] [--plan-only=1] [--hard-selector=<selector>] [--selector-bypass=1]",
        desc: "Experimental runtime assimilation lane. Requires Node.js 22+ full surface; known targets route to governed core bridges, unknown targets fail as unadmitted unless local simulation is explicitly enabled.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::RuntimeScript,
        script_rel: "client/runtime/systems/tools/assimilation_cli_bridge.ts",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "session <status|register|resume|send|list>",
        desc: "Manage command-center sessions.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://command-center-session",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "orchestration",
        desc: "Rust-core orchestration invoke surface (coordinator/scratchpad/checkpoint).",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://orchestration",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "swarm-runtime",
        desc: "Core swarm runtime lanes.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "capability-profile",
        desc: "Show hardware-sensed capability shedding profile.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://capability-profile",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:spawn",
        desc: "Spawn a governed swarm session.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:send",
        desc: "Send inter-agent message between sessions.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:receive",
        desc: "Receive pending inter-agent messages.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:ack",
        desc: "Acknowledge inter-agent message delivery.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:handoff",
        desc: "Perform a governed inter-agent handoff with lineage and context receipts.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:context",
        desc: "Put/get governed swarm context variables.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:bootstrap",
        desc: "Fetch the authoritative direct-send/budget bootstrap contract for a session.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:state",
        desc: "Inspect session state/context/tool history.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:query",
        desc: "Query swarm service discovery/result registry.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:sessions:tick",
        desc: "Advance persistent swarm check-ins.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:tools:register",
        desc: "Register a JSON-schema tool manifest over the governed swarm bridge.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:tools:invoke",
        desc: "Invoke a governed swarm tool manifest.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:stream:emit",
        desc: "Emit delimited swarm stream chunks with receipt anchors.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:stream:render",
        desc: "Render delimited swarm stream chunks for an agent turn.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:turns:run",
        desc: "Run a governed multi-turn swarm execution with fail-closed recovery.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:turns:show",
        desc: "Inspect a governed multi-turn swarm run receipt.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:networks:create",
        desc: "Create a composable governed swarm agent network.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:networks:status",
        desc: "Inspect a governed swarm agent network receipt.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "autonomy:swarm:demo",
        desc: "Run the optional thin swarm REPL/demo shell over the governed bridge.",
        tier: CommandTier::Experimental,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://swarm-runtime",
        read_only: false,
        unsafe_surface: true,
    },
    CommandItem {
        synopsis: "version",
        desc: "Print runtime version and build info.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://version-cli",
        read_only: true,
        unsafe_surface: false,
    },
    CommandItem {
        synopsis: "update",
        desc: "Check for the latest release and print upgrade guidance.",
        tier: CommandTier::Tier1,
        handler: CommandHandlerKind::CoreDomain,
        script_rel: "core://version-cli",
        read_only: true,
        unsafe_surface: false,
    },
];
