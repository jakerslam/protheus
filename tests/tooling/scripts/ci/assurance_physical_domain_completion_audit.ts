#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const DEFAULT_OUT = 'core/local/artifacts/assurance_physical_domain_completion_audit_current.json';

type Failure = {
  id: string;
  path: string;
  detail: string;
  category?: string;
};

type Mirror = {
  legacy_path?: string;
  canonical_path?: string;
  reason?: string;
  removal_target?: string;
};

function isRetiredMirrorRegistry(payload: any): boolean {
  return payload.status === 'retired'
    && payload.burn_down_required === false
    && typeof payload.retired_at === 'string'
    && payload.retired_at.length > 0
    && typeof payload.retired_reason === 'string'
    && payload.retired_reason.length > 0;
}

type DebtRow = {
  id?: string;
  kind?: string;
  offender_path?: string;
  offender_path_prefix?: string;
  canonical_path?: string;
  owner?: string;
  todo_id?: string;
  severity?: string;
  expires?: string;
};

const CANONICAL_PREFIXES = ['validation/', 'observability/'];
const DEBT_REGISTER = 'validation/conformance/contracts/assurance_physical_domain_debt_register.json';
const EXEMPTION_REGISTER = 'validation/conformance/contracts/assurance_physical_domain_placement_exemptions.json';

const REQUIRED_DOMAIN_MANIFESTS = [
  {
    domain: 'validation',
    path: 'validation/domain_manifest.json',
    owner: 'assurance.validation',
    requiredSubdomains: [
      'validation/tests',
      'validation/evals',
      'validation/benchmarks',
      'validation/conformance',
      'validation/regression',
      'validation/release_gates',
      'validation/scorecards',
      'validation/governance',
      'validation/schemas',
      'validation/fixtures',
      'validation/reports',
    ],
  },
  {
    domain: 'observability',
    path: 'observability/domain_manifest.json',
    owner: 'assurance.observability',
    requiredSubdomains: [
      'observability/telemetry',
      'observability/health',
      'observability/traces',
      'observability/sentinel',
      'observability/runtime_findings',
      'observability/evidence_normalization',
      'observability/freshness',
      'observability/source_coverage',
    ],
  },
];

const REQUIRED_CANONICAL_DEFINITIONS = [
  'validation/evals/config/eval_quality_thresholds.json',
  'validation/evals/policies/eval_issue_candidate_dedupe_policy.json',
  'validation/evals/policies/live_eval_monitor_policy.json',
  'validation/evals/policies/eval_authority_calibration_policy.json',
  'validation/evals/policies/eval_feedback_lifecycle_policy.json',
  'validation/evals/policies/eval_issue_filing_policy.json',
  'validation/evals/policies/live_eval_policy.json',
  'validation/evals/contracts/eval_issue_patch_links.json',
  'validation/evals/contracts/eval_issue_taxonomy.json',
  'observability/contracts/self_hosted_deploy_contract_v1.json',
  'observability/contracts/incident_response_contract_v1.json',
  'observability/contracts/workflow_editor_contract_v1.json',
  'observability/contracts/realtime_monitoring_contract_v1.json',
  'validation/evals/contracts/eval_loop_contract_v1.json',
  'validation/reports/client_archive/proof_pack_latest.json',
  'validation/reports/client_archive/benchmark_matrix_run_latest.json',
  'validation/evals/fixtures/eval_adversarial_matrix.json',
  'validation/evals/fixtures/eval_gold_dataset_seed.jsonl',
  'validation/evals/schemas/eval_gold_dataset.schema.json',
  'validation/evals/fixtures/eval_gold_dataset_v1.jsonl',
  'validation/evals/fixtures/eval_learning_loop_review_labels.jsonl',
  'validation/evals/fixtures/eval_action_economy_cases.json',
  'validation/evals/fixtures/eval_adversarial_routing_cases.json',
  'validation/evals/fixtures/eval_contamination_cases.json',
  'validation/evals/fixtures/eval_fix_verification_before.json',
  'validation/evals/fixtures/eval_fix_verification_after.json',
  'validation/evals/fixtures/eval_grader_hacking_cases.json',
  'validation/evals/fixtures/eval_holdout_red_team_cases.json',
  'validation/evals/fixtures/eval_issue_taxonomy_v1.json',
  'validation/evals/fixtures/eval_learning_loop_policy_holdout.json',
  'validation/evals/fixtures/eval_learning_loop_traces.json',
  'validation/evals/fixtures/eval_metamorphic_cases.json',
  'validation/evals/fixtures/eval_multiturn_simulation_cases.json',
  'validation/evals/fixtures/eval_phase_trace_sample.json',
  'validation/evals/fixtures/eval_production_workflow_telemetry.json',
  'validation/evals/fixtures/eval_quality_thresholds.json',
  'validation/evals/fixtures/eval_reviewer_feedback_sample.jsonl',
  'validation/evals/fixtures/eval_rsi_promotion_ladder.json',
  'validation/evals/fixtures/eval_trace_localization_cases.json',
  'validation/evals/fixtures/eval_trajectory_scoring_cases.json',
  'validation/evals/fixtures/eval_workflow_selection_cases.json',
  'validation/evals/fixtures/synthetic_user_chat_harness_cases.json',
  'validation/release_gates/config/release_gates.yaml',
  'validation/release_gates/contracts/release_proof_pack_manifest.json',
  'validation/release_gates/policies/release_blocker_rubric.json',
  'validation/release_gates/proof_packs/2026-04-25/manifest.json',
  'validation/scorecards/contracts/release_scorecard_contract.json',
  'validation/scorecards/contracts/assurance_scorecard_derivation_contract.json',
  'validation/governance/contracts/assurance_governance_registry.json',
  'validation/benchmarks/policies/runtime_boundedness_budgets.json',
  'validation/benchmarks/policies/benchmark_regression_budgets.json',
  'validation/benchmarks/fixtures/public_harness_workloads.json',
  'validation/regression/policies/runtime_empirical_coverage_policy.json',
  'validation/regression/policies/runtime_soak_scenarios_policy.json',
  'validation/conformance/contracts/assurance_validation_registry.json',
  'validation/conformance/contracts/assurance_consumer_boundary_contract.json',
  'validation/conformance/contracts/assurance_physical_domain_placement_exemptions.json',
  'validation/conformance/contracts/assurance_physical_domain_debt_register.json',
  'validation/schemas/assurance_validation_registry.schema.json',
  'validation/schemas/assurance_consumer_boundary_contract.schema.json',
  'validation/schemas/assurance_governance_registry.schema.json',
  'observability/source_coverage/assurance_observability_registry.json',
  'observability/source_coverage/assurance_observability_registry.schema.json',
  'observability/telemetry/competitive_benchmark_adaptive_index.json',
  'observability/dashboards/metrics-spec.json',
  'observability/deploy/docker-compose.yml',
  'observability/deploy/defaults.json',
  'validation/scorecards/policies/ci_quality_scorecard_policy.json',
  'validation/release_gates/policies/release_gate_canary_rollback_enforcer_policy.json',
  'validation/release_gates/policies/error_budget_release_gate_policy.json',
  'validation/evals/policies/gold_eval_blind_scoring_policy.json',
  'validation/benchmarks/policies/scale_benchmark_policy.json',
  'validation/benchmarks/policies/mobile_competitive_benchmark_matrix_policy.json',
  'validation/benchmarks/policies/competitive_observability_benchmark_policy.json',
  'validation/benchmarks/policies/competitive_benchmark_matrix_policy.json',
  'validation/benchmarks/policies/benchmark_sanity_policy.json',
  'validation/benchmarks/policies/benchmark_autonomy_gate_policy.json',
  'validation/benchmarks/fixtures/competitive_benchmark_snapshot_2026_02.json',
  'observability/telemetry/rust_observability_parity_contract.json',
  'observability/telemetry/runtime_telemetry_policy.json',
  'observability/telemetry/observability_policy.json',
  'observability/telemetry/observability_deployment_defaults_policy.json',
  'observability/health/sre_observability_contract.json',
  'observability/health/observability_slo_runbook_closure_policy.json',
  'observability/freshness/world_model_freshness_policy.json',
  'observability/freshness/surface_embodiment_freshness_policy.json',
  'observability/freshness/memory_index_freshness_policy.json',
  'observability/evidence_normalization/deny_telemetry_normalizer_policy.json',
  'observability/dashboards/enterprise_slo_observability_dashboard_policy.json',
  'observability/telemetry/persona_telemetry.jsonl',
  'observability/telemetry/persona_telemetry_policy.json',
  'observability/evidence_normalization/assurance_evidence_envelope.schema.json',
  'observability/freshness/evidence_freshness_policy.json',
  'observability/health/health_stream_contract.json',
  'observability/traces/sentinel_trace_source_map.json',
  'observability/runtime_findings/runtime_finding.schema.json',
  'observability/sentinel/sentinel_resident_observer_contract.json',
];

const ASSURANCE_GATE_ROWS = [
  'ops:assurance:placement:guard',
  'ops:assurance:envelope:guard',
  'ops:assurance:scorecard-derivation:guard',
  'ops:assurance:shell-truth-leak:guard',
  'ops:assurance:physical-domain-placement:guard',
  'ops:assurance:physical-domain-completion:audit',
];

function flag(name: string): string | undefined {
  const prefix = `--${name}=`;
  const match = process.argv.find((arg) => arg.startsWith(prefix));
  return match ? match.slice(prefix.length) : undefined;
}

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/').replace(/^\.\//, '');
}

function exists(rel: string): boolean {
  return fs.existsSync(path.resolve(ROOT, rel));
}

function readJson<T = any>(rel: string): T {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, rel), 'utf8')) as T;
}

function writeJson(rel: string, payload: unknown): void {
  const abs = path.resolve(ROOT, rel);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`);
}

function isCanonicalPath(rel: string): boolean {
  const normalized = normalizePath(rel);
  return CANONICAL_PREFIXES.some((prefix) => normalized.startsWith(prefix));
}

function debtKey(kind: string, pathOrPrefix: string): string {
  return `${kind}:${normalizePath(pathOrPrefix)}`;
}

function discoverCompatibilityMirrors(): string[] {
  const found: string[] = [];
  const stack = ['validation', 'observability'];
  while (stack.length) {
    const rel = stack.pop()!;
    const abs = path.resolve(ROOT, rel);
    if (!fs.existsSync(abs)) continue;
    for (const entry of fs.readdirSync(abs, { withFileTypes: true })) {
      const child = normalizePath(path.join(rel, entry.name));
      if (entry.isDirectory()) {
        stack.push(child);
      } else if (entry.name === 'compatibility_mirrors.json') {
        found.push(child);
      }
    }
  }
  return found.sort();
}

function scanDomains(): Failure[] {
  const failures: Failure[] = [];
  for (const manifestSpec of REQUIRED_DOMAIN_MANIFESTS) {
    if (!exists(manifestSpec.path)) {
      failures.push({
        id: 'missing_domain_manifest',
        path: manifestSpec.path,
        category: manifestSpec.domain,
        detail: 'domain manifest is required for physical-domain completion',
      });
      continue;
    }
    const manifest = readJson<any>(manifestSpec.path);
    if (manifest.domain !== manifestSpec.domain) {
      failures.push({
        id: 'domain_manifest_wrong_domain',
        path: manifestSpec.path,
        category: manifestSpec.domain,
        detail: `expected domain ${manifestSpec.domain}, found ${String(manifest.domain)}`,
      });
    }
    if (manifest.owner !== manifestSpec.owner) {
      failures.push({
        id: 'domain_manifest_wrong_owner',
        path: manifestSpec.path,
        category: manifestSpec.domain,
        detail: `expected owner ${manifestSpec.owner}, found ${String(manifest.owner)}`,
      });
    }
    for (const subdomain of manifestSpec.requiredSubdomains) {
      if (!exists(subdomain)) {
        failures.push({
          id: 'missing_domain_subdirectory',
          path: subdomain,
          category: manifestSpec.domain,
          detail: 'required physical subdomain directory is missing',
        });
      }
      const readme = `${subdomain}/README.md`;
      if (!exists(readme)) {
        failures.push({
          id: 'missing_domain_subdirectory_anchor',
          path: readme,
          category: manifestSpec.domain,
          detail: 'required physical subdomain README anchor is missing',
        });
      }
    }
  }
  return failures;
}

function scanCanonicalDefinitions(): Failure[] {
  const failures: Failure[] = [];
  const injectedMissing = flag('inject-missing-canonical');
  for (const rel of REQUIRED_CANONICAL_DEFINITIONS) {
    if (rel === injectedMissing || !exists(rel)) {
      failures.push({
        id: 'missing_canonical_definition',
        path: rel,
        detail: 'canonical Assurance definition is missing from Validation/Observability physical domain',
      });
      continue;
    }
    if (!isCanonicalPath(rel)) {
      failures.push({
        id: 'canonical_definition_outside_domain',
        path: rel,
        detail: 'canonical definition must live under validation/** or observability/**',
      });
    }
  }
  return failures;
}

function scanCompatibilityMirrors(): {
  failures: Failure[];
  mirrorFiles: string[];
  mirrorRows: number;
  legacyPaths: string[];
  mirrors: Array<{ legacy: string; canonical: string; source: string }>;
} {
  const failures: Failure[] = [];
  const mirrorFiles = discoverCompatibilityMirrors();
  let mirrorRows = 0;
  const legacyPaths: string[] = [];
  const mirrors: Array<{ legacy: string; canonical: string; source: string }> = [];
  for (const file of mirrorFiles) {
    const payload = readJson<any>(file);
    const fileMirrors = Array.isArray(payload.mirrors) ? payload.mirrors as Mirror[] : [];
    const retiredRegistry = isRetiredMirrorRegistry(payload);
    if (!retiredRegistry && (!payload.burn_down_required || typeof payload.burn_down_todo !== 'string' || payload.burn_down_todo.length === 0)) {
      failures.push({
        id: 'mirror_registry_missing_burn_down_marker',
        path: file,
        detail: 'active compatibility mirror registry must declare burn_down_required=true and burn_down_todo; empty retired registries require status=retired, burn_down_required=false, retired_at, and retired_reason',
      });
    }
    if (fileMirrors.length === 0 && !retiredRegistry) {
      failures.push({
        id: 'mirror_registry_empty',
        path: file,
        detail: 'compatibility mirror registry must contain at least one old-location row unless explicitly retired',
      });
    }
    for (const mirror of fileMirrors) {
      mirrorRows += 1;
      const legacy = mirror.legacy_path ? normalizePath(mirror.legacy_path) : '';
      const canonical = mirror.canonical_path ? normalizePath(mirror.canonical_path) : '';
      if (!legacy || !canonical || !mirror.reason) {
        failures.push({
          id: 'invalid_compatibility_mirror_row',
          path: file,
          detail: 'each mirror row requires legacy_path, canonical_path, and reason',
        });
        continue;
      }
      legacyPaths.push(legacy);
      mirrors.push({ legacy, canonical, source: file });
      if (isCanonicalPath(legacy)) {
        failures.push({
          id: 'compatibility_mirror_legacy_path_is_canonical',
          path: legacy,
          detail: 'legacy path should identify an old scattered location, not the canonical domain',
        });
      }
      if (!isCanonicalPath(canonical)) {
        failures.push({
          id: 'compatibility_mirror_does_not_point_inward',
          path: file,
          detail: `${legacy} points to non-canonical path ${canonical}`,
        });
      }
      if (!exists(canonical)) {
        failures.push({
          id: 'compatibility_mirror_canonical_target_missing',
          path: canonical,
          detail: `${legacy} mirror target does not exist`,
        });
      }
      if (!mirror.removal_target && !payload.burn_down_todo) {
        failures.push({
          id: 'compatibility_mirror_missing_row_burn_down',
          path: file,
          detail: `${legacy} needs row-level removal_target or registry-level burn_down_todo`,
        });
      }
    }
  }
  if (process.argv.includes('--inject-bad-mirror=1') || process.argv.includes('--inject-bad-mirror')) {
    failures.push({
      id: 'compatibility_mirror_does_not_point_inward',
      path: 'tests/tooling/config/injected_legacy_eval_policy.json',
      detail: 'controlled negative: injected mirror target points outside Validation/Observability',
    });
  }
  return { failures, mirrorFiles, mirrorRows, legacyPaths, mirrors };
}

function scanRegisteredDebt(mirrors: Array<{ legacy: string; canonical: string; source: string }>): {
  failures: Failure[];
  knownViolations: DebtRow[];
  debtRows: number;
} {
  const failures: Failure[] = [];
  const today = new Date().toISOString().slice(0, 10);
  if (!exists(DEBT_REGISTER)) {
    return {
      failures: [{ id: 'missing_physical_domain_debt_register', path: DEBT_REGISTER, detail: 'known physical-domain debt must be registered as violations with TODO rows' }],
      knownViolations: [],
      debtRows: 0,
    };
  }
  const rows = (readJson<any>(DEBT_REGISTER).violations || []) as DebtRow[];
  const byKey = new Map<string, DebtRow>();
  for (const row of rows) {
    const pathOrPrefix = row.offender_path || row.offender_path_prefix || '';
    if (!row.id || !row.kind || !pathOrPrefix || !row.owner || !row.todo_id || row.severity !== 'registered_violation' || !row.expires) {
      failures.push({ id: 'invalid_physical_domain_debt_row', path: pathOrPrefix || DEBT_REGISTER, detail: 'debt rows require id, kind, offender path/prefix, owner, todo_id, severity=registered_violation, and expires' });
      continue;
    }
    if (row.expires < today) {
      failures.push({ id: 'expired_physical_domain_debt_row', path: pathOrPrefix, detail: `${row.id} expired ${row.expires}` });
    }
    byKey.set(debtKey(row.kind, pathOrPrefix), row);
  }
  const knownViolations: DebtRow[] = [];
  for (const mirror of mirrors) {
    const row = byKey.get(debtKey('compatibility_mirror', mirror.legacy));
    if (!row) {
      failures.push({ id: 'unregistered_compatibility_mirror_violation', path: mirror.legacy, detail: 'compatibility mirror debt must have a matching debt-register TODO row' });
      continue;
    }
    if (normalizePath(String(row.canonical_path || '')) !== normalizePath(mirror.canonical)) {
      failures.push({ id: 'debt_register_canonical_mismatch', path: mirror.legacy, detail: `${row.id} canonical path does not match mirror target` });
    }
    knownViolations.push(row);
  }
  const exemptions = (readJson<any>(EXEMPTION_REGISTER).exemptions || []) as Array<{ path?: string; path_prefix?: string }>;
  for (const exemption of exemptions) {
    const pathOrPrefix = normalizePath(exemption.path || exemption.path_prefix || '');
    const row = byKey.get(debtKey('placement_exemption', pathOrPrefix));
    if (!row) {
      failures.push({ id: 'unregistered_placement_exemption_violation', path: pathOrPrefix, detail: 'placement exemption debt must have a matching debt-register TODO row' });
      continue;
    }
    knownViolations.push(row);
  }
  if (process.argv.includes('--inject-unregistered-debt=1') || process.argv.includes('--inject-unregistered-debt')) {
    failures.push({ id: 'unregistered_compatibility_mirror_violation', path: 'tests/tooling/config/injected_unregistered_eval_policy.json', detail: 'controlled negative: unregistered physical-domain debt' });
  }
  return { failures, knownViolations, debtRows: rows.length };
}

function scanConsumerWiring(legacyPaths: string[]): Failure[] {
  const failures: Failure[] = [];
  const pkg = readJson<any>('package.json');
  const scripts = pkg.scripts || {};
  const registry = readJson<any>('tests/tooling/config/tooling_gate_registry.json');
  const gates = registry.gates || registry;

  for (const gateId of ASSURANCE_GATE_ROWS) {
    if (!scripts[gateId]) {
      failures.push({
        id: 'package_script_missing_assurance_gate',
        path: 'package.json',
        detail: `${gateId} is not wired as an npm script`,
      });
    }
    const row = gates[gateId];
    if (!row) {
      failures.push({
        id: 'tooling_registry_missing_assurance_gate',
        path: 'tests/tooling/config/tooling_gate_registry.json',
        detail: `${gateId} is missing from tooling registry`,
      });
      continue;
    }
    const canonical = Array.isArray(row.canonical_definition_paths) ? row.canonical_definition_paths.map(String) : [];
    if (canonical.length === 0) {
      failures.push({
        id: 'tooling_registry_missing_canonical_definition_paths',
        path: 'tests/tooling/config/tooling_gate_registry.json',
        detail: `${gateId} must declare canonical_definition_paths`,
      });
    }
    for (const defPath of canonical) {
      if (!isCanonicalPath(defPath)) {
        failures.push({
          id: 'tooling_registry_canonical_definition_outside_domain',
          path: 'tests/tooling/config/tooling_gate_registry.json',
          detail: `${gateId} references ${defPath}`,
        });
      }
      if (!exists(defPath)) {
        failures.push({
          id: 'tooling_registry_canonical_definition_missing',
          path: defPath,
          detail: `${gateId} references a missing canonical definition path`,
        });
      }
    }
  }

  for (const [scriptName, command] of Object.entries(scripts)) {
    if (!scriptName.startsWith('ops:assurance:')) continue;
    const text = String(command);
    for (const legacyPath of legacyPaths) {
      if (text.includes(legacyPath)) {
        failures.push({
          id: 'assurance_consumer_uses_legacy_definition_path',
          path: 'package.json',
          detail: `${scriptName} still references ${legacyPath}`,
        });
      }
    }
  }
  return failures;
}

function run(): void {
  const strict = process.argv.includes('--strict') || process.argv.includes('--strict=1');
  const out = flag('out') || DEFAULT_OUT;
  const domainFailures = scanDomains();
  const canonicalFailures = scanCanonicalDefinitions();
  const mirrorScan = scanCompatibilityMirrors();
  const debtScan = scanRegisteredDebt(mirrorScan.mirrors);
  const consumerFailures = scanConsumerWiring(mirrorScan.legacyPaths);
  const failOnKnownViolations = process.argv.includes('--fail-on-known-violations=1') || process.argv.includes('--fail-on-known-violations');
  const knownViolationFailures = failOnKnownViolations && debtScan.knownViolations.length > 0
    ? [{ id: 'known_physical_domain_violations_remain', path: DEBT_REGISTER, detail: `${debtScan.knownViolations.length} registered physical-domain violations remain` }]
    : [];
  const failures = [...domainFailures, ...canonicalFailures, ...mirrorScan.failures, ...debtScan.failures, ...consumerFailures, ...knownViolationFailures];
  const payload = {
    ok: failures.length === 0,
    type: 'assurance_physical_domain_completion_audit',
    generated_at: new Date().toISOString(),
    strict,
    summary: {
      required_domain_manifests: REQUIRED_DOMAIN_MANIFESTS.length,
      required_canonical_definitions: REQUIRED_CANONICAL_DEFINITIONS.length,
      compatibility_mirror_files: mirrorScan.mirrorFiles.length,
      compatibility_mirror_rows: mirrorScan.mirrorRows,
      registered_debt_rows: debtScan.debtRows,
      known_violation_count: debtScan.knownViolations.length,
      clean_without_known_violations: debtScan.knownViolations.length === 0,
      assurance_gate_rows: ASSURANCE_GATE_ROWS.length,
      domain_failures: domainFailures.length,
      canonical_failures: canonicalFailures.length,
      mirror_failures: mirrorScan.failures.length,
      debt_register_failures: debtScan.failures.length,
      consumer_failures: consumerFailures.length,
      failures: failures.length,
    },
    artifact_paths: [out],
    mirror_files: mirrorScan.mirrorFiles,
    known_violations: debtScan.knownViolations,
    failures,
  };
  writeJson(out, payload);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && failures.length) process.exit(1);
}

run();
