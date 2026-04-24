#!/usr/bin/env tsx

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-MEMORY-CONTINUITY-CLOSURE-001';
const LEGACY_SRS_IDS = ['V11-MEMORY-003'];
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PROFILES = 'tests/tooling/config/verify_profiles.json';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const PACKAGE_JSON = 'package.json';
const OUT_JSON = 'core/local/artifacts/memory_continuity_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/MEMORY_CONTINUITY_CLOSURE_GUARD_CURRENT.md';
const GATE_ID = 'ops:memory-continuity:closure:guard';

const ACTIVE_CONTEXT_SOURCE = 'core/layer0/ops/src/dashboard_compat_api_parts/031-context-window-and-recall_parts/010-message-token-cost.rs';
const HISTORICAL_CONTEXT_SOURCE = 'core/layer0/ops/src/dashboard_compat_api_parts/031-context-window-and-recall_parts/020-historical-context-keyframes-prompt-context.rs';
const MESSAGE_CONTEXT_SOURCE = 'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/030-message-routing-and-context.rs';
const MESSAGE_PAYLOAD_SOURCE = 'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/050-message-finalization-and-payload_parts/050-finalize-message-finalization-and-payload_parts/040-segment-004.rs';
const MEMORY_REGRESSION_ONE = 'core/layer0/ops/src/dashboard_compat_api_parts/config_payload_tests_parts/060-context-telemetry-and-auto-compact_parts/030-memory-recall-stays-scoped-to-active-session-history.rs';
const MEMORY_REGRESSION_TWO = 'core/layer0/ops/src/dashboard_compat_api_parts/config_payload_tests_parts/100-governance-and-semantic-memory_parts/040-part_parts/002-part.rs';
const DREAM_PRELUDE = 'core/layer0/ops/src/autonomy_controller_parts/010-prelude-and-state.rs.parts/010-segment.rs';
const DREAM_DEFAULT_STATE = 'core/layer0/ops/src/autonomy_controller_parts/050-proactive-daemon-dream-speculation_parts/001-part_parts/010-proactive-daemon-default-state-to-proactive-daemon-tool-receipts.rs';
const DREAM_SCHEDULER = 'core/layer0/ops/src/autonomy_controller_parts/050-proactive-daemon-dream-speculation_parts/002-part_parts/020-segment-002.rs';
const DREAM_EXECUTOR = 'core/layer0/ops/src/autonomy_controller_parts/050-proactive-daemon-dream-speculation_parts/002-part_parts/030-segment-003.rs';
const DREAM_COMMAND = 'core/layer0/ops/src/autonomy_controller_parts/040-command-dispatch.rs';
const DREAM_REGRESSION = 'core/layer0/ops/src/autonomy_controller_parts/051-speculation-and-regression-tests.regression_tests_parts/030-proactive-daemon-policy-tiered-tool-surfaces-emit-conduit-receip.rs';
const SRS = 'docs/workspace/SRS.md';
const TODO = 'docs/workspace/TODO.md';

type Check = { id: string; ok: boolean; detail?: string };

function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  const match = process.argv.find((item) => item.startsWith(prefix));
  return match ? match.slice(prefix.length) : fallback;
}

function flag(name: string, fallback: boolean): boolean {
  const value = arg(name, fallback ? '1' : '0').toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

function readText(path: string): string {
  return readFileSync(path, 'utf8');
}

function readJson(path: string): any {
  return JSON.parse(readText(path));
}

function list(value: any): string[] {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

function check(id: string, ok: boolean, detail?: string): Check {
  return detail ? { id, ok, detail } : { id, ok };
}

function missingTokens(source: string, tokens: string[]): string[] {
  return tokens.filter((token) => !source.includes(token));
}

function readMany(paths: string[]): string {
  return paths.map((path) => `${path}\n${readText(path)}`).join('\n---\n');
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function packageScript(pkg: any, name: string): string {
  const value = pkg?.scripts?.[name];
  return typeof value === 'string' ? value : '';
}

function registryGate(registry: any, gateId: string): any {
  return registry?.gates?.[gateId];
}

function registryRunnable(registry: any, gateId: string): boolean {
  const entry = registryGate(registry, gateId);
  return Boolean(entry && (Array.isArray(entry.command) || typeof entry.script === 'string'));
}

function registryArtifacts(registry: any, gateId: string): string[] {
  return list(registryGate(registry, gateId)?.artifact_paths);
}

function profileGateIds(profiles: any, profile: string): string[] {
  return list(profiles?.profiles?.[profile]?.gate_ids);
}

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function workloadArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.workload_and_quality);
}

function optionalReports(manifest: any): string[] {
  return list(manifest?.optional_reports);
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Memory Continuity Closure Guard',
    '',
    `- pass: ${pass}`,
    `- srs_id: ${SRS_ID}`,
    `- legacy_srs_ids: ${LEGACY_SRS_IDS.join(', ')}`,
    '',
    '| Check | Status | Detail |',
    '| --- | --- | --- |',
    ...checks.map((row) => `| ${row.id} | ${row.ok ? 'pass' : 'fail'} | ${row.detail ?? ''} |`),
    '',
  ];
  writeFileSync(path, lines.join('\n'));
}

function main(): void {
  const registryPath = arg('registry', REGISTRY);
  const profilesPath = arg('profiles', PROFILES);
  const manifestPath = arg('manifest', MANIFEST);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const strict = flag('strict', true);

  const registry = readJson(registryPath);
  const profiles = readJson(profilesPath);
  const manifest = readJson(manifestPath);
  const pkg = readJson(PACKAGE_JSON);
  const activeContext = readMany([ACTIVE_CONTEXT_SOURCE, HISTORICAL_CONTEXT_SOURCE, MESSAGE_CONTEXT_SOURCE, MESSAGE_PAYLOAD_SOURCE]);
  const memoryTests = readMany([MEMORY_REGRESSION_ONE, MEMORY_REGRESSION_TWO]);
  const dreamSources = readMany([DREAM_PRELUDE, DREAM_DEFAULT_STATE, DREAM_SCHEDULER, DREAM_EXECUTOR, DREAM_COMMAND]);
  const dreamTests = readText(DREAM_REGRESSION);
  const srs = readText(SRS);
  const todo = readText(TODO);
  const required = requiredArtifacts(manifest);
  const workload = workloadArtifacts(manifest);
  const reports = optionalReports(manifest);

  const activeContextTokens = [
    'enforce_recent_context_floor',
    'historical_context_keyframes_prompt_context',
    'Long-thread keyframes outside the active window',
    'recent_floor_active_satisfied',
    'recent_floor_continuity_status',
    'recent_floor_continuity_action',
    'recent_floor_continuity_message',
    'recent_floor_target',
    'recent_floor_missing_before',
    'recent_floor_coverage_after',
  ];
  const memoryTestTokens = [
    'context_command_reports_recent_floor_reinjection_when_pool_trim_is_aggressive',
    'recent_floor_continuity_status',
    'recent_floor_continuity_action',
    'recent_floor_enforcement_rehydrates_tail_after_pool_trim',
    'relevant_recall_uses_full_history_even_when_pool_drops_older_facts',
  ];
  const dreamTokens = [
    '--dream-idle-ms',
    '--dream-max-without-ms',
    'dream_consolidation',
    'last_dream_at_ms',
    'run_dream_consolidation_for_hand',
    'crate::spine::execute_sleep_cleanup',
    'last_cleanup_ok',
    '"dream" => run_dream_consolidation',
  ];
  const dreamTestTokens = [
    'proactive_daemon_triggers_dream_and_cleanup_when_inactive',
    '--dream-idle-ms=60000',
    '--dream-max-without-ms=60000',
    'dream_consolidation',
    'last_dream_at_ms',
    'sleep cleanup should run as part of dream execution',
  ];

  const missingActiveContext = missingTokens(activeContext, activeContextTokens);
  const missingMemoryTests = missingTokens(memoryTests, memoryTestTokens);
  const missingDream = missingTokens(dreamSources, dreamTokens);
  const missingDreamTests = missingTokens(dreamTests, dreamTestTokens);

  const checks: Check[] = [
    check('package_script_present', packageScript(pkg, GATE_ID).includes('tooling:run') && packageScript(pkg, GATE_ID).includes(GATE_ID), packageScript(pkg, GATE_ID)),
    check('registry_gate_runnable', registryRunnable(registry, GATE_ID), GATE_ID),
    check('registry_artifacts_registered', registryArtifacts(registry, GATE_ID).includes(outJson) && registryArtifacts(registry, GATE_ID).includes(outMarkdown), registryArtifacts(registry, GATE_ID).join(', ')),
    check('fast_profile_covers_gate', profileGateIds(profiles, 'fast').includes(GATE_ID), GATE_ID),
    check('boundary_profile_covers_gate', profileGateIds(profiles, 'boundary').includes(GATE_ID), GATE_ID),
    check('release_profile_covers_gate', profileGateIds(profiles, 'release').includes(GATE_ID), GATE_ID),
    check('artifact_required', required.includes(outJson), outJson),
    check('artifact_workload_grouped', workload.includes(outJson), outJson),
    check('report_listed', reports.includes(outMarkdown), outMarkdown),
    check('active_context_floor_and_backfill_tokens_present', missingActiveContext.length === 0, missingActiveContext.join(', ')),
    check('memory_context_regression_tokens_present', missingMemoryTests.length === 0, missingMemoryTests.join(', ')),
    check('dream_inactivity_and_cleanup_tokens_present', missingDream.length === 0, missingDream.join(', ')),
    check('dream_cleanup_regression_tokens_present', missingDreamTests.length === 0, missingDreamTests.join(', ')),
    check('srs_closure_row_present', srs.includes(SRS_ID) && srs.includes(GATE_ID), SRS_ID),
    check('todo_legacy_rows_closed', !todo.includes('- [ ] `V11-MEMORY-003`'), 'all V11-MEMORY-003 TODO rows must be checked'),
  ];

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'memory_continuity_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: LEGACY_SRS_IDS,
    generated_at: new Date().toISOString(),
    gate_id: GATE_ID,
    artifacts: { json: outJson, markdown: outMarkdown },
    source_contracts: {
      active_context_floor: ACTIVE_CONTEXT_SOURCE,
      historical_backfill: HISTORICAL_CONTEXT_SOURCE,
      message_context: MESSAGE_CONTEXT_SOURCE,
      message_payload: MESSAGE_PAYLOAD_SOURCE,
      dream_scheduler: DREAM_SCHEDULER,
      dream_executor: DREAM_EXECUTOR,
    },
    regression_contracts: {
      memory_context: [MEMORY_REGRESSION_ONE, MEMORY_REGRESSION_TWO],
      dream_cleanup: DREAM_REGRESSION,
    },
    checks,
  };

  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);

  if (!pass) {
    const failed = checks.filter((row) => !row.ok).map((row) => row.id).join(', ');
    console.error(`memory continuity closure guard failed: ${failed}`);
    if (strict) process.exitCode = 1;
    return;
  }
  console.log(`memory continuity closure guard passed: ${outJson}`);
}

main();
