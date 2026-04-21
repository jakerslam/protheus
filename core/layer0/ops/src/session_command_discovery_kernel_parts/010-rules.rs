const IGNORED_EXACT: &[&str] = &["", "true", "false", ":"];
const IGNORED_PREFIXES: &[&str] = &["#", "echo ", "printf "];

const GIT_SUBCMD_SAVINGS: &[(&str, f64)] = &[
    ("status", 94.0),
    ("log", 88.0),
    ("diff", 86.0),
    ("show", 84.0),
    ("checkout", 80.0),
    ("switch", 80.0),
    ("pull", 72.0),
    ("push", 72.0),
    ("commit", 74.0),
];

const GIT_SUBCMD_STATUS: &[(&str, SupportStatus)] = &[
    ("status", SupportStatus::Existing),
    ("log", SupportStatus::Existing),
    ("diff", SupportStatus::Existing),
    ("show", SupportStatus::Existing),
    ("checkout", SupportStatus::Passthrough),
    ("switch", SupportStatus::Passthrough),
    ("pull", SupportStatus::Passthrough),
    ("push", SupportStatus::Passthrough),
    ("commit", SupportStatus::Passthrough),
];

const CARGO_SUBCMD_SAVINGS: &[(&str, f64)] = &[
    ("test", 90.0),
    ("check", 86.0),
    ("build", 84.0),
    ("fmt", 80.0),
    ("clippy", 80.0),
];

const CARGO_SUBCMD_STATUS: &[(&str, SupportStatus)] = &[
    ("test", SupportStatus::Existing),
    ("check", SupportStatus::Existing),
    ("build", SupportStatus::Existing),
    ("fmt", SupportStatus::Passthrough),
    ("clippy", SupportStatus::Passthrough),
];

const RULES: &[DiscoverRule] = &[
    DiscoverRule {
        pattern: r"^(?:infring\s+)?git\s+([a-zA-Z0-9_-]+)(?:\s|$)",
        canonical: "infring git",
        category: "Git",
        savings_pct: 82.0,
        subcmd_savings: GIT_SUBCMD_SAVINGS,
        subcmd_status: GIT_SUBCMD_STATUS,
    },
    DiscoverRule {
        pattern: r"^(?:infring\s+)?cargo\s+([a-zA-Z0-9_-]+)(?:\s|$)",
        canonical: "infring cargo",
        category: "Cargo",
        savings_pct: 78.0,
        subcmd_savings: CARGO_SUBCMD_SAVINGS,
        subcmd_status: CARGO_SUBCMD_STATUS,
    },
    DiscoverRule {
        pattern: r"^(?:infring\s+)?(pytest|cargo\s+test|npm\s+test|pnpm\s+test)(?:\s|$)",
        canonical: "infring tests",
        category: "Tests",
        savings_pct: 84.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^(?:infring\s+)?(rg|find|ls|cat|head|tail|wc)(?:\s|$)",
        canonical: "infring files",
        category: "Files",
        savings_pct: 72.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
    DiscoverRule {
        pattern: r"^(?:infring\s+)?(gh|curl|wget)(?:\s|$)",
        canonical: "infring network",
        category: "Network",
        savings_pct: 70.0,
        subcmd_savings: &[],
        subcmd_status: &[],
    },
];
