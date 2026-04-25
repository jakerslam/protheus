pub(super) fn print_usage() {
    println!("Usage:");
    println!("  infring-ops runtime-efficiency-floor run [--strict=1|0] [--policy=<path>]");
    println!("  infring-ops runtime-efficiency-floor status [--policy=<path>]");
    println!("  infring-ops benchmark-autonomy-gate <run|status> [--strict=1|0]");
    println!("  infring-ops approval-gate-kernel <status|queue|approve|deny|was-approved|parse-command|parse-yaml|replace> [flags]");
    println!("  infring-ops conduit-client-security-kernel <build-security|resolve-security-config> --payload-base64=<base64_json>");
    println!("  infring-ops collector-runtime-kernel <classify-error|resolve-controls|begin-collection|prepare-run|finalize-run|prepare-attempt|mark-success|mark-failure> --payload-base64=<base64_json>");
    println!("  infring-ops collector-state-kernel <meta-load|meta-save|cadence-check|cache-load|cache-save> --payload-base64=<base64_json>");
    println!("  infring-ops collector-content-kernel <extract-entries|extract-json-rows|map-feed-items|map-json-items> --payload-base64=<base64_json>");
    println!("  infring-ops stock-market-collector-kernel <prepare-run|build-fetch-plan|finalize-run|extract-quotes|map-quotes|fallback-indices|collect> --payload-base64=<base64_json>");
    println!("  infring-ops moltbook-hot-collector-kernel <preflight|classify-fetch-error|map-posts|collect> --payload-base64=<base64_json>");
    println!("  infring-ops moltstack-discover-collector-kernel <preflight|build-fetch-plan|classify-fetch-error|finalize-run|map-posts|collect> --payload-base64=<base64_json>");
    println!("  infring-ops bird-x-collector-kernel <preflight|prepare-run|map-results|finalize-run|collect> --payload-base64=<base64_json>");
    println!("  infring-ops upwork-gigs-collector-kernel <run|prepare-run|build-fetch-plan|finalize-run|parse-rss|map-gigs|fallback-gigs|collect> --payload-base64=<base64_json>");
    println!("  infring-ops github-repo-collector-kernel <run|resolve-run-params|resolve-auth|prepare-repo-activity|build-repo-activity-fetch-plan|finalize-repo-activity|collect-repo-activity|build-pr-review-fetch-plan|build-pr-review|collect-pr-review|file-risk-flags> --payload-base64=<base64_json>");
    println!("  infring-ops assimilate-kernel <target> [--json=1] [--showcase=1] [--duration-ms=<n>] [--scaffold-payload=1] [--allow-local-simulation=1] [--plan-only=1] [--hard-selector=<selector>] [--selector-bypass=1] [--target=<name>] [--core-domain=<domain>] [--core-args-base64=<base64_json_array>]");
    println!("  infring-ops security-layer-inventory-gate-kernel <run|status> [--strict=1|0] [--write=1|0]");
    println!(
        "  infring-ops rust-hotpath-inventory-kernel <run|status|inventory> [--policy=<path>]"
    );
    println!(
        "  infring-ops top50-roi-sweep-kernel <run|queue|status> [--max=<n>] [--policy=<path>]"
    );
    println!(
        "  infring-ops top200-roi-sweep-kernel <run|queue|status> [--max=<n>] [--policy=<path>]"
    );
    println!("  infring-ops passport-iteration-chain-kernel <record|status> [--payload-base64=<base64_json>]");
    println!("  infring-ops egress-gateway-kernel <load-policy|load-state|authorize> [--payload-base64=<base64_json>]");
    println!("  infring-ops web-conduit|browse <status|receipts|setup|migrate-legacy-config|native-codex|fetch|search|providers> [flags]");
    println!("  infring-ops batch-query <query|status|policy> [--source=web] [--query=<terms>] [--aperture=small|medium]");
    println!("  infring-ops memory-semantic-query <query|status> [--agent=<id>] [--query=<terms>] [--limit=<n>]");
    println!("  infring-ops context-stacks <create|list|archive|tail-merge|tail-promote|render|batch-class|scheduler-check|node-spike|contract-verify|taste-tune|partial-merge|hybrid-retrieve|status|policy> [flags]");
    println!("  infring-ops stomach-kernel <run|status|rollback|retention|purge> [flags]");
    println!("  infring-ops session-command-session-analytics-kernel <extract-jsonl|classify-jsonl|adoption-report> [--payload=<json>|--payload-base64=<base64_json>]");
    println!("  infring-ops session-command-discovery-kernel <classify|classify-text> [--payload=<json>|--payload-base64=<base64_json>]");
    println!("  infring-ops session-command-tracking-kernel <record|summary|status> [--payload=<json>|--payload-base64=<base64_json>]");
    println!("  infring-ops command-permission-kernel <evaluate|match-pattern|extract-pattern> [--payload=<json>|--payload-base64=<base64_json>]");
    println!("  infring-ops source-comment-filter-kernel <filter|detect-language> [--payload=<json>|--payload-base64=<base64_json>]");
    println!("  infring-ops workspace-file-search <search|list|mention|status> [--workspace=<path>] [--workspace-roots-json='[...]'] [--workspace-hint=<name>] [--q=<query>] [--type=file|folder]");
    println!("  infring-ops adaptive-layer-store-kernel <paths|is-within-root|resolve-path|resolve-provider-artifact-path|read-json|ensure-json|set-json|delete-path> [--payload-base64=<base64_json>]");
    println!("  infring-ops catalog-store-kernel <paths|default-state|normalize-state|read-state|ensure-state|set-state> [--payload-base64=<base64_json>]");
    println!("  infring-ops focus-trigger-store-kernel <paths|default-state|normalize-state|read-state|ensure-state|set-state> [--payload-base64=<base64_json>]");
    println!("  infring-ops security-integrity-kernel <load-policy|collect-present-files|verify|seal|append-event> [--payload-base64=<base64_json>]");
    println!("  infring-ops queue-sqlite-kernel <open|ensure-schema|migrate-history|upsert-item|append-event|insert-receipt|queue-stats|backpressure-policy> [--payload-base64=<base64_json>]");
    println!("  infring-ops benchmark-matrix <run|status> [--snapshot=<path>] [--refresh-runtime=1|0] [--bar-width=44] [--throughput-uncached=1|0] [--benchmark-preflight=1|0] [--preflight-max-load-per-core=0.90] [--preflight-max-noise-cv-pct=12.5] [--preflight-noise-sample-ms=250] [--preflight-noise-rounds=3]");
    println!("  infring-ops fixed-microbenchmark <run|status> [--rounds=9] [--warmup-runs=2] [--sample-ms=800] [--work-factor=16] [--workload-id=sha256_fixed_workload_v1] [--provider-contract=webFetchProviders] [--provider-plugin-id=<id>]");
    println!("  infring-ops f100-reliability-certification <run|status> [--strict=1|0] [--policy=<path>]");
    println!("  infring-ops sdlc-change-control <run|status> [--strict=1|0] [--policy=<path>] [--pr-body-path=<path>] [--changed-paths-path=<path>]");
    println!("  infring-ops system-health-audit-runner-kernel <run|status> [--strict=1|0] [--policy=<path>]");
    println!("  infring-ops kernel-sentinel <run|status|report|help> [--strict=1|0] [--findings-path=<path>]");
    println!("  infring-ops supply-chain-provenance-v2 <run|status> [--strict=1|0] [--policy=<path>] [--bundle-path=<path>] [--vuln-summary-path=<path>]");
    println!("  infring-ops f100-readiness-program <run|run-all|status> [--lane=<V6-F100-XXX>] [--strict=1|0] [--apply=1|0] [--policy=<path>]");
    println!("  infring-ops identity-federation <authorize|scim-lifecycle|status> [flags]");
    println!("  infring-ops audit-log-export <export|status> [flags]");
    println!("  infring-ops model-router <args>");
    println!("  infring-ops intelligence-nexus <status|open|add-key|credits-status|buy-credits|autobuy-evaluate> [flags]");
    println!("  infring-ops network-protocol <status|ignite-bitcoin|stake|merkle-root|emission|zk-claim> [flags]");
    println!("  infring-ops seed-protocol <status|deploy|migrate|enforce|select|archive|defend|monitor> [flags]");
    println!("  infring-ops binary-blob-runtime <status|migrate|settle|mutate|substrate-probe|debug-access> [flags]");
    println!("  infring-ops directive-kernel <status|dashboard|prime-sign|derive|supersede|compliance-check|bridge-rsi|migrate> [flags]");
    println!("  infring-ops action-envelope-kernel <create|classify|auto-classify|requires-approval|detect-irreversible|generate-id> [--payload-base64=<base64_json>]");
    println!("  infring-ops action-receipts-kernel <now-iso|append-jsonl|with-receipt-contract|write-contract-receipt|replay-task-lineage|query-task-lineage> [--payload-base64=<base64_json>] [--task-id=<id>] [--trace-id=<id>] [--limit=<n>] [--scan-root=<path>] [--sources=<csv_paths>]");
    println!("  infring-ops conversation-eye-synthesizer-kernel <synthesize-envelope> [--payload-base64=<base64_json>]");
    println!("  infring-ops conversation-eye-collector-kernel <begin-collection|preflight|load-source-rows|normalize-topics|load-index|apply-node|process-nodes|append-memory-row|append-memory-rows|save-index> [--payload-base64=<base64_json>]");
    println!("  infring-ops trainability-matrix-kernel <default-policy|normalize-policy|load-policy|evaluate> [--payload-base64=<base64_json>]");
    println!("  infring-ops dynamic-burn-budget-signal-kernel <normalize-pressure|pressure-rank|cost-pressure|load-signal> [--payload-base64=<base64_json>]");
    println!("  infring-ops policy-runtime-kernel <deep-merge|resolve-policy-path|load-policy-runtime|resolve-policy-value-path> [--payload-base64=<base64_json>]");
    println!("  infring-ops camel-bridge <status|register-society|run-society|simulate-world|import-dataset|route-conversation|record-crab-benchmark|register-tool-gateway|invoke-tool-gateway|record-scaling-observation|assimilate-intake> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops semantic-kernel-bridge <status|register-service|register-plugin|invoke-plugin|collaborate|plan|register-vector-connector|retrieve|register-llm-connector|route-llm|validate-structured-output|emit-enterprise-event|register-dotnet-bridge|invoke-dotnet-bridge|run-governed-workflow> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops google-adk-bridge <status|register-a2a-agent|send-a2a-message|run-llm-agent|register-tool-manifest|invoke-tool-manifest|coordinate-hierarchy|approval-checkpoint|rewind-session|record-evaluation|sandbox-execute|deploy-shell|register-runtime-bridge|route-model> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops haystack-bridge <status|register-pipeline|run-pipeline|run-agent-toolset|register-template|render-template|register-document-store|retrieve-documents|route-and-rank|record-multimodal-eval|trace-run|import-connector|assimilate-intake> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops workflow_graph-bridge <status|register-graph|checkpoint-run|inspect-state|interrupt-run|resume-run|coordinate-subgraph|record-trace|stream-graph|run-governed-workflow> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>] [--trace-path=<path>]");
    println!("  infring-ops workflow_chain-bridge <status|register-chain|execute-chain|register-middleware|run-deep-agent|register-memory-bridge|recall-memory|import-integration|route-prompt|parse-structured-output|record-trace|checkpoint-run|assimilate-intake> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops crewai-bridge <status|register-crew|run-process|run-flow|memory-bridge|ingest-config|route-delegation|review-crew|record-amp-trace|benchmark-parity|route-model|run-governed-workflow> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>] [--approval-queue-path=<path>] [--trace-path=<path>]");
    println!("  infring-ops shannon-bridge <status|register-pattern|guard-budget|memory-bridge|replay-run|approval-checkpoint|sandbox-execute|record-observability|gateway-route|register-tooling|schedule-run|desktop-shell|p2p-reliability|assimilate-intake> [--payload-base64=<base64_json>] [--state-path=<path>] [--history-path=<path>] [--approval-queue-path=<path>] [--replay-dir=<path>] [--observability-trace-path=<path>] [--observability-metrics-path=<path>] [--desktop-history-path=<path>]");
    println!("  infring-ops instinct-bridge <status|cold-start-model|activate|refine> [--payload-base64=<base64_json>] [--state-path=<path>] [--history-path=<path>] [--lineage-path=<path>]");
    println!("  infring-ops baremetal-substrate <status|boot-kernel|schedule|memory-manager|fs-driver|network-stack|security-model> [--payload-base64=<base64_json>] [--state-path=<path>] [--history-path=<path>] [--ledger-path=<path>]");
    println!("  infring-ops phone-runtime-bridge <status|battery-schedule|sensor-intake|interaction-mode|background-daemon|phone-profile> [--payload-base64=<base64_json>] [--state-path=<path>] [--history-path=<path>] [--background-state-path=<path>] [--sensor-state-path=<path>]");
    println!("  infring-ops pydantic-ai-bridge <status|register-agent|validate-output|register-tool-context|invoke-tool-context|bridge-protocol|durable-run|approval-checkpoint|record-logfire|execute-graph|stream-model|record-eval|assimilate-intake|register-runtime-bridge|route-model|run-governed-workflow> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops dspy-bridge <status|register-signature|compile-program|optimize-program|assert-program|import-integration|execute-multihop|record-benchmark|record-optimization-trace|assimilate-intake> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops mastra-bridge <status|register-graph|execute-graph|run-agent-loop|memory-recall|suspend-run|resume-run|register-mcp-bridge|invoke-mcp-bridge|record-eval-trace|deploy-shell|register-runtime-bridge|route-model|scaffold-intake|run-governed-workflow> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops llamaindex-bridge <status|register-index|query|run-agent-workflow|ingest-multimodal|record-memory-eval|run-conditional-workflow|emit-trace|register-connector|connector-query> [--payload-base64=<base64_json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops integrity-hash-utility-kernel <stable-stringify|sha256-hex|hash-file-sha256> [--payload-base64=<base64_json>]");
    println!("  infring-ops redaction-classification-kernel <load-policy|classify-text|redact-text|classify-and-redact> [--payload-base64=<base64_json>]");
    println!("  infring-ops runtime-path-registry-kernel <constants|normalize-for-root|resolve-canonical|resolve-client-state|resolve-core-state> [--payload-base64=<base64_json>]");
    println!("  infring-ops proposal-type-classifier-kernel <normalize-type-key|extract-source-eye-id|classify> [--payload-base64=<base64_json>]");
    println!("  infring-ops state-artifact-contract-kernel <now-iso|decorate-artifact-row|trim-jsonl-rows|write-artifact-set|append-artifact-history> [--payload-base64=<base64_json>]");
    println!("  infring-ops success-criteria-kernel <status|parse-rows|evaluate> [flags]");
    println!("  infring-ops success-criteria-compiler-kernel <compile-rows|compile-proposal|to-action-spec-rows> [--payload-base64=<base64_json>]");
    println!("  infring-ops outcome-fitness-kernel <load-policy|normalize-threshold-overrides|normalize-ranking-weights|normalize-proposal-type-threshold-offsets|normalize-promotion-policy-overrides|normalize-value-currency-policy-overrides|normalize-proposal-type-key|normalize-value-currency-token|proposal-type-threshold-offsets-for> [--payload-base64=<base64_json>]");
    println!("  infring-ops training-conduit-schema-kernel <default-policy|normalize-policy|load-policy|build-metadata|validate-metadata> [--payload-base64=<base64_json>]");
    println!("  infring-ops tool-response-compactor-kernel <compact|redact|extract-summary> [--payload-base64=<base64_json>]");
    println!("  infring-ops trit-kernel <normalize|label|from-label|invert|majority|consensus|propagate|serialize|parse-serialized|serialize-vector|parse-vector> [--payload-base64=<base64_json>]");
    println!("  infring-ops request-envelope-kernel <envelope-payload|canonical-string|sign|verify|stamp-env|verify-from-env|normalize-files|normalize-key-id|secret-key-env-var-name> [--payload-base64=<base64_json>]");
    println!("  infring-ops autonomy-receipt-schema-kernel <to-success-criteria-record|with-success-criteria-verification|normalize-receipt|success-criteria-from-receipt> [--payload-base64=<base64_json>]");
    println!("  infring-ops uid-kernel <normalize-prefix|is-alnum|stable-uid|random-uid> [--payload-base64=<base64_json>]");
    println!("  infring-ops quorum-validator-kernel <evaluate> [--payload-base64=<base64_json>]");
    println!("  infring-ops mutation-provenance-kernel <load-policy|normalize-meta|enforce|record-audit> [--payload-base64=<base64_json>]");
    println!("  infring-ops ops-domain-conduit-runner-kernel <parse-argv|build-pass-args|build-run-options|prepare-run> [--payload-base64=<base64_json>]");
    println!("  infring-ops spine-conduit-bridge-kernel <run-domain|normalize-spine-args> [--domain=<name>] [--normalize-spine=1|0] [-- <args...>]");
    println!("  infring-ops local-runtime-partitioner <status|init|reset> [--workspace-root=<path>] [--confirm=RESET_LOCAL]");
    println!("  infring-ops local-state-digest-kernel <preflight|collect> [--payload-base64=<base64_json>]");
    println!("  infring-ops strategy-store-kernel <paths|default-state|default-draft|normalize-mode|normalize-execution-mode|normalize-profile|validate-profile|normalize-queue-item|recommend-mode|read-state|ensure-state|set-state|upsert-profile|intake-signal|materialize-from-queue|touch-profile-usage|evaluate-gc-candidates|gc-profiles> [--payload-base64=<base64_json>]");
    println!("  infring-ops habit-store-kernel <default-state|normalize-state|read-state|ensure-state|set-state> [--payload-base64=<base64_json>]");
    println!("  infring-ops reflex-store-kernel <default-state|normalize-state|read-state|ensure-state|set-state> [--payload-base64=<base64_json>]");
    println!("  infring-ops strategy-campaign-scheduler-kernel <normalize-campaigns|annotate-priority|build-decomposition-plans> [--payload-base64=<base64_json>]");
    println!("  infring-ops queued-backlog-kernel <ensure-dir|read-json|write-json-atomic|append-jsonl|read-jsonl|resolve-path|stable-hash|load-policy> [--payload-base64=<base64_json>]");
    println!("  infring-ops upgrade-lane-kernel <status|record> [--payload-base64=<base64_json>]");
    println!("  infring-ops mech-suit-mode-kernel <load-policy|approx-token-count|classify-severity|should-emit-console|update-status|append-attention-event> [--payload-base64=<base64_json>]");
    println!("  infring-ops rsi-ignition <status|ignite|reflect|swarm|evolve> [flags]");
    println!(
        "  infring-ops continuity-runtime <resurrection-protocol|session-continuity-vault> [flags]"
    );
    println!("  infring-ops memory-plane <causal-temporal-graph|memory-federation-plane> [flags]");
    println!("  infring-ops memory-policy-kernel <status|parse-cli|command-name|validate|validate-ranking|validate-lensmap|severity-rank|guard-failure> [--payload-base64=<base64_json>]");
    println!("  infring-ops memory-session-isolation-kernel <load-state|save-state|validate|failure-result> [--payload-base64=<base64_json>]");
    println!(
        "  infring-ops readiness-bridge-pack-kernel <run|status> [--strict=1|0] [--policy=<path>]"
    );
    println!("  infring-ops runtime-systems <status|verify|run|build|manifest|bootstrap|package|settle> [flags]");
    println!("  infring-ops child-organ-runtime <plan|spawn|status> [flags]");
    println!("  infring-ops organism-layer <status|ignite|dream|homeostasis|crystallize|symbiosis|mutate|sensory|narrative> [flags]");
    println!("  infring-ops graph-toolkit <status|pagerank|louvain|jaccard|label-propagation|betweenness|predict-links|centrality|communities> [flags]");
    println!("  infring-ops asm-plane <status|wasm-dual-meter|hands-runtime|crdt-adapter|trust-chain|fastpath|industrial-pack> [flags]");
    println!("  infring-ops research-plane <status|diagnostics|fetch|recover-selectors|crawl|mcp-extract|spider|crawl-spider|middleware|crawl-middleware|pipeline|crawl-pipeline|signals|crawl-signals|console|crawl-console|template-governance|goal-crawl|map-site|extract-structured|monitor|firecrawl-template-governance|js-scrape|auth-session|proxy-rotate|parallel-scrape-workers|book-patterns-template-governance|decode-news-url|decode-news-urls|decoder-template-governance> [flags]");
    println!("  infring-ops parse-plane <status|parse-doc|visualize|postprocess-table|flatten|export|template-governance> [flags]");
    println!("  infring-ops flow-plane <status|compile|playground|component-marketplace|export|template-governance> [flags]");
    println!("  infring-ops app-plane <status|run|history|replay|switch-provider|build|ingress|template-governance> [flags]");
    println!("  infring-ops snowball-plane <status|start|melt-refine|compact|backlog-pack|control> [flags]");
    println!("  infring-ops mcp-plane <status|capability-matrix|workflow|expose|pattern-pack|template-governance> [flags]");
    println!("  infring-ops skills-plane <status|list|dashboard|create|activate|chain-validate|install|run|share|gallery|react-minimal|tot-deliberate> [flags]");
    println!("  infring-ops vbrowser-plane <status|session-start|session-control|automate|privacy-guard> [flags]");
    println!("  infring-ops agency-plane <status|create-shadow|topology|orchestrate|workflow-bind> [flags]");
    println!(
        "  infring-ops collab-plane <status|dashboard|launch-role|schedule|continuity> [flags]"
    );
    println!("  infring-ops company-plane <status|orchestrate-agency|budget-enforce|ticket|heartbeat> [flags]");
    println!("  infring-ops business-plane <taxonomy|persona|continuity|alerts|switchboard|external-sync|continuity-audit|archive|status> [flags]");
    println!("  infring-ops canyon-plane <efficiency|hands-army|evolution|sandbox|ecosystem|workflow|scheduler|control-plane|adoption|benchmark-gate|status> [flags]");
    println!("  infring-ops government-plane <attestation|classification|nonrepudiation|diode|soc|coop|proofs|interoperability|ato-pack|status> [flags]");
    println!("  infring-ops finance-plane <transaction|model-governance|aml|kyc|finance-eye|risk-warehouse|custody|zero-trust|availability|regulatory-report|status> [flags]");
    println!("  infring-ops healthcare-plane <patient|phi-audit|cds|devices|documentation|alerts|coordination|trials|imaging|emergency|status> [flags]");
    println!("  infring-ops vertical-plane <activate|compile-profile|status> [flags]");
    println!("  infring-ops nexus-plane <package-domain|bridge|insurance|human-boundary|receipt-v2|merkle-forest|compliance-ledger|status> [flags]");
    println!("  infring-ops substrate-plane <status|csi-capture|csi-module|csi-embedded-profile|csi-policy|eye-bind|bio-interface|bio-feedback|bio-adapter-template|bioethics-policy|bio-enable> [flags]");
    println!(
        "  infring-ops observability-plane <status|monitor|workflow|incident|selfhost> [flags]"
    );
    println!("  infring-ops persist-plane <status|schedule|mobile-cockpit|continuity|connector|cowork> [flags]");
    println!("  infring-ops binary-vuln-plane <status|scan|mcp-analyze> [flags]");
    println!("  infring-ops hermes-plane <status|discover|continuity|delegate|cockpit> [flags]");
    println!(
        "  infring-ops eval-plane <status|enable-neuralavb|experiment-loop|benchmark|run> [flags]"
    );
    println!("  infring-ops ab-lane-eval <status|run> [flags]");
    println!("  infring-ops contract-check <args>");
    println!("  infring-ops security-plane <guard|anti-sabotage-shield|constitution-guardian|remote-emergency-halt|soul-token-guard|integrity-reseal|integrity-reseal-assistant|capability-lease|startup-attestation|truth-seeking-gate|abac-policy-plane|status> [flags]");
    println!("  infring-ops enterprise-hardening <run|status|export-compliance|identity-surface|certify-scale|dashboard> [flags]");
    println!("  infring-ops rollout-rings <status|evaluate> [flags]");
    println!("  infring-ops strategy-mode-governor <args>");
    println!(
        "  infring-ops strategy-resolver <status|invoke> [--payload=<json>|--payload-file=<path>]"
    );
    println!("  infring-ops status [--dashboard]");
    println!("  infring-ops dashboard <start|status|snapshot|runtime-sync> [--dashboard-host=<ip>] [--dashboard-port=<n>] [--team=<id>] [--refresh-ms=<n>] (dashboard-ui is internal compat alias)");
    println!("  infring-ops daemon-control <start|stop|restart|status|heal|attach|subscribe|tick|diagnostics|watchdog> [flags]");
    println!("  infring-ops verity-plane <status|drift-status|vector-check|record-event|refine-event> [flags]");
    println!("  infring-ops command-center-session <register|resume|send|status|list> [flags]");
    println!("  infring-ops command-list-kernel [--mode=<list|help>] [--json]");
    println!("  infring-ops operator-tooling-kernel <status|route-model|escalate-model|plan-auto|plan-validate|postflight-validate|output-validate|state-read|state-write|decision-log-append|append-decision|safe-apply|memory-search|memory-summarize|memory-last-change|membrief|trace-find|sync-allowed-models|smoke-routing|spawn-safe|smart-spawn|auto-spawn|execute-handoff|safe-run|control_runtime-health|daily-brief|cron-drift|cron-sync|doctor|audit-plane|fail-playbook> [flags]");
    println!("  infring-ops coverage-badge-kernel [run] [--ts=<path>] [--rust=<path>] [--contracts=<path>] [--out-json=<path>] [--out-badge=<path>]");
    println!("  infring-ops organ-atrophy-controller <scan|status|revive> [flags]");
    println!("  infring-ops narrow-agent-parity-harness <run|status> [flags]");
    println!("  infring-ops offsite-backup <sync|restore-drill|status|diagnose|list> [flags]");
    println!("  infring-ops settlement-program <list|run|run-all|settle|revert|edit-core|edit-module|status> [flags]");
    println!("  infring-ops llm-economy-organ <run|enable|dashboard|status> [flags]");
    println!("  infring-ops metakernel <status|registry|manifest|worlds|capability-taxonomy|budget-admission|epistemic-object|effect-journal|substrate-registry|radix-guard|quantum-broker|neural-consent|attestation-graph|degradation-contracts|execution-profiles|variant-profiles|mpu-compartments|microkernel-safety|dna-status|dna-create|dna-mutate|dna-enforce-subservience|dna-hybrid-status|dna-hybrid-commit|dna-hybrid-verify|dna-hybrid-repair-gene|dna-hybrid-worm-supersede|dna-hybrid-worm-mutate|dna-hybrid-protected-lineage|invariants> [flags]");
    println!("  infring-ops top1-assurance <status|proof-coverage|proof-vm|size-gate|benchmark-thresholds|tooling-contracts|comparison-matrix|run-all> [flags]");
    println!("  infring-ops backlog-queue-executor <run|status> [flags]");
    println!("  infring-ops backlog-delivery-plane <run|status> [--id=<Vx-...>] [flags]");
    println!("  infring-ops backlog-runtime-anchor <build|verify> --lane-id=<V3-RACE-XXX>");
    println!("  infring-ops legacy-retired-lane <build|verify> --lane-id=<SYSTEMS-OPS-...>");
    println!("  infring-ops inversion-controller <command> [flags]");
    println!("  infring-ops health-status <command> [flags]");
    println!("  infring-ops alpha-readiness <run|status> [--strict=1|0] [--run-gates=1|0]");
    println!("  infring-ops foundation-contract-gate <run|status> [flags]");
    println!(
        "  infring-ops origin-integrity <run|status|certificate|seed-bootstrap-verify> [flags]"
    );
    println!("  infring-ops state-kernel <command> [flags]");
    println!("  infring-ops shadow-budget-governance <evaluate|status> [flags]");
    println!("  infring-ops adaptive-runtime <tick|status> [flags]");
    println!("  infring-ops adaptive-intelligence <status|propose|shadow-train|prioritize|graduate> [flags]");
    println!("  infring-ops offline-runtime-guard <evaluate|status> [flags]");
    println!("  infring-ops hardware-route-hardening <evaluate|status> [flags]");
    println!("  infring-ops autonomy-controller <command> [flags]");
    println!("  infring-ops autotest-controller <command> [flags]");
    println!("  infring-ops autotest-doctor <command> [flags]");
    println!("  infring-ops autonomy-proposal-enricher <command> [flags]");
    println!("  infring-ops spine <mode> [date] [flags]");
    println!("  infring-ops attention-queue <enqueue|status> [flags]");
    println!("  infring-ops memory-ambient <run|status> [flags]");
    println!("  infring-ops duality-seed <status|invoke> [--payload=<json>|--payload-file=<path>]");
    println!("  infring-ops persona-ambient <apply|status> [flags]");
    println!("  infring-ops dopamine-ambient <closeout|status|evaluate> [flags]");
    println!("  infring-ops persona-schema-contract <validate|status> [--strict=1|0] [--schema-mode=<id>] [--payload=<json>|--input=<path>]");
    println!("  infring-ops infringctl <command> [flags]");
    println!("  infring-ops infringd-launcher-kernel gate [--payload-base64=<base64_json>]");
    println!("  infring-ops rag <status|start|ingest|search|chat|merge-vault|memory> [flags]");
    println!("  infring-ops personas-cli <command> [flags]");
    println!(
        "  infring-ops autophagy-auto-approval <evaluate|monitor|commit|rollback|status> [flags]"
    );
    println!("  infring-ops adaptive-contract-version-governance <run|status> [flags]");
    println!("  infring-ops assimilation-controller <command> [flags]");
    println!("  infring-ops collector-cache <load|save|status> [flags]");
    println!("  infring-ops contribution-oracle <validate|status> [flags]");
    println!("  infring-ops sensory-eyes-intake <command> [flags]");
    println!("  infring-ops spawn-broker <status|request|release> [flags]");
    println!(
        "  infring-ops swarm-runtime <status|spawn|byzantine-test|consensus-check|test> [flags]"
    );
    println!("  infring-ops execution-yield-recovery <command> [flags]");
    println!("  infring-ops infring-control-plane <command> [flags]");
    println!("  infring-ops rust50-migration-program <command> [flags]");
    println!("  infring-ops venom-containment-layer <command> [flags]");
    println!("  infring-ops dynamic-burn-budget-oracle <command> [flags]");
    println!("  infring-ops backlog-registry <command> [flags]");
    println!("  infring-ops rust-enterprise-productivity-program <command> [flags]");
    println!("  infring-ops backlog-github-sync <command> [flags]");
    println!("  infring-ops workflow-controller <command> [flags]");
    println!("  infring-ops workflow-executor <command> [flags]");
    println!("  infring-ops fluxlattice-program <list|run|run-all|status> [flags]");
    println!("  infring-ops perception-polish-program <list|run|run-all|status> [flags]");
    println!("  infring-ops scale-readiness-program <list|run|run-all|status> [flags]");
    println!("  infring-ops opendev-dual-agent <run|status> [flags]");
    println!("  infring-ops company-layer-orchestration <run|status> [flags]");
    println!("  infring-ops wifi-csi-engine <run|status> [flags]");
    println!("  infring-ops biological-computing-adapter <run|status> [flags]");
    println!("  infring-ops dify-bridge <status|register-canvas|sync-knowledge-base|register-agent-app|publish-dashboard|route-provider|run-conditional-flow|record-audit-trace> [flags]");
    println!("  infring-ops metagpt-bridge <status|register-company|run-sop|simulate-pr|run-debate|plan-requirements|record-oversight|record-pipeline-trace|ingest-config> [flags]");
    println!("  infring-ops observability-automation-engine <workflow|status> [flags]");
    println!("  infring-ops observability-slo-runbook-closure <incident|status> [flags]");
    println!("  infring-ops persistent-background-runtime <schedule|status> [flags]");
    println!("  infring-ops workspace-gateway-runtime <run|status> [flags]");
    println!("  infring-ops p2p-gossip-seed <run|status> [flags]");
    println!("  infring-ops startup-agency-builder <run|status> [flags]");
    println!("  infring-ops timeseries-receipt-engine <run|status> [flags]");
    println!("  infring-ops webgpu-inference-adapter <run|status> [flags]");
    println!("  infring-ops context-doctor <run|status> [flags]");
    println!("  infring-ops discord-swarm-orchestration <run|status> [flags]");
    println!("  infring-ops bookmark-knowledge-pipeline <run|status> [flags]");
    println!("  infring-ops public-api-catalog <status|sync|search|integrate|connect|import-flow|run-flow|verify> [flags]");
    println!("  infring-ops decentralized-data-marketplace <run|status> [flags]");
    println!("  infring-ops autoresearch-loop <run|status> [flags]");
    println!("  infring-ops intel-sweep-router <run|status> [flags]");
    println!("  infring-ops nexus-internal-comms <status|validate|compress|decompress|send|log|agent-prompt|resolve-modules|export-lexicon> [flags]");
    println!("  infring-ops gui-drift-manager <run|status> [flags]");
    println!("  infring-ops release-gate-canary-rollback-enforcer <gate|status> [flags]");
    println!("  infring-ops srs-contract-runtime <run|run-many|status> [--id=<V6-...>|--ids=<csv>|--ids-file=<path>] [flags]");
}
