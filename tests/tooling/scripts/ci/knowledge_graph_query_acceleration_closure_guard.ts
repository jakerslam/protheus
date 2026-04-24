#!/usr/bin/env tsx

import { mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-KNOWLEDGE-GRAPH-QUERY-ACCELERATION-CLOSURE-001';
const LEGACY_SRS_IDS = [
  'V11-MEMORY-010.1',
  'V11-MEMORY-010.2',
  'V11-MEMORY-010.3',
  'V11-MEMORY-010.4',
  'V11-MEMORY-010.5',
];
const GATE_ID = 'ops:knowledge-graph-query-acceleration:closure:guard';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PROFILES = 'tests/tooling/config/verify_profiles.json';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const PACKAGE_JSON = 'package.json';
const OUT_JSON = 'core/local/artifacts/knowledge_graph_query_acceleration_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/KNOWLEDGE_GRAPH_QUERY_ACCELERATION_CLOSURE_GUARD_CURRENT.md';

const STATE_SOURCE = 'core/layer2/memory/src/graph_query_acceleration_state.rs';
const TYPES_SOURCE = 'core/layer2/memory/src/graph_query_acceleration_types.rs';
const PATHS_SOURCE = 'core/layer2/memory/src/graph_query_acceleration_paths.rs';
const QUERY_SOURCE = 'core/layer2/memory/src/graph_query_acceleration_query.rs';
const RUNTIME_SOURCE = 'core/layer2/memory/src/graph_query_acceleration_runtime.rs';
const SUBSYSTEM_SOURCE = 'core/layer2/memory/src/graph_subsystem.rs';
const REGRESSION_SOURCE = 'core/layer2/memory/src/graph_query_acceleration_tests.rs';
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
    '# Knowledge Graph Query Acceleration Closure Guard',
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
  const stateAndSubsystem = readMany([STATE_SOURCE, SUBSYSTEM_SOURCE]);
  const typeAndPathSources = readMany([TYPES_SOURCE, PATHS_SOURCE]);
  const queryAndRuntime = readMany([QUERY_SOURCE, RUNTIME_SOURCE]);
  const regressions = readText(REGRESSION_SOURCE);
  const srs = readText(SRS);
  const todo = readText(TODO);
  const required = requiredArtifacts(manifest);
  const workload = workloadArtifacts(manifest);
  const reports = optionalReports(manifest);

  const multiIndexTokens = [
    'pub(crate) struct TripleIndexes',
    'pub spo: BTreeMap',
    'pub sop: BTreeMap',
    'pub pso: BTreeMap',
    'pub pos: BTreeMap',
    'pub osp: BTreeMap',
    'pub ops: BTreeMap',
    'pub predicate_bitmaps',
    'pub relation_source_bloom',
    'relation_bitmap_and',
    'relation_source_might_exist',
    'register_edge',
  ];
  const pathTokens = [
    'pub enum GraphTraversalAlgorithm',
    'Bfs',
    'Dfs',
    'Dijkstra',
    'AStar',
    'Bidirectional',
    'pub fn find_path',
    'fn bfs_path',
    'fn dfs_path',
    'fn dijkstra_path',
    'fn a_star_path',
    'fn bidirectional_path',
  ];
  const plannerTokens = [
    'plan_triple_query',
    'estimate_pattern_cardinality',
    'selectivity_first_with_leapfrog_domains',
    'build_variable_domains',
    'leapfrog_intersection',
    'execute_triple_query',
    'cache_get_fresh',
    'cache_get_seed',
    'cache_put',
    'cache_status',
  ];
  const materializationTokens = [
    'materialize_transitive_closure',
    'neighborhood_summary',
    'materialize_inference_edges',
    'sample_subgraph',
    'GraphSamplingStrategy',
    'approximate_tail_candidates',
    'rebuild_entity_embeddings_if_needed',
    'ann_buckets',
  ];
  const federationTokens = [
    'GraphPartitionStrategy',
    'GraphPartitionPlan',
    'FederatedServiceProfile',
    'FederatedDispatchStep',
    'FederatedQueryPlan',
    'build_partition_plan',
    'build_federated_query_plan',
    'selectivity_hint_bps',
  ];
  const regressionTokens = [
    'planner_prefers_selective_pattern_order',
    'query_execution_uses_cache_and_returns_bindings',
    'bitmap_and_bloom_filters_surface_relation_candidates',
    'path_algorithms_and_materialized_views_work',
    'neighborhood_inference_sampling_embedding_and_partitioning_work',
    'federated_plan_assigns_patterns_to_best_service',
  ];

  const missingMultiIndex = missingTokens(stateAndSubsystem, multiIndexTokens);
  const missingPaths = missingTokens(typeAndPathSources, pathTokens);
  const missingPlanner = missingTokens(queryAndRuntime, plannerTokens);
  const missingMaterialization = missingTokens(queryAndRuntime, materializationTokens);
  const missingFederation = missingTokens(typeAndPathSources + '\n' + queryAndRuntime, federationTokens);
  const missingRegressions = missingTokens(regressions, regressionTokens);
  const openLegacyRows = LEGACY_SRS_IDS.filter((id) => todo.includes(`- [ ] \`${id}\``));
  const srsRowsDone = LEGACY_SRS_IDS.filter((id) => srs.includes(`| ${id} | done |`));

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
    check('multi_index_lookup_tokens_present', missingMultiIndex.length === 0, missingMultiIndex.join(', ')),
    check('path_algorithm_tokens_present', missingPaths.length === 0, missingPaths.join(', ')),
    check('planner_join_cache_tokens_present', missingPlanner.length === 0, missingPlanner.join(', ')),
    check('materialization_approximation_tokens_present', missingMaterialization.length === 0, missingMaterialization.join(', ')),
    check('partition_federation_tokens_present', missingFederation.length === 0, missingFederation.join(', ')),
    check('regression_tests_present', missingRegressions.length === 0, missingRegressions.join(', ')),
    check('srs_closure_row_present', srs.includes(SRS_ID) && srs.includes(GATE_ID), SRS_ID),
    check('legacy_srs_rows_done', srsRowsDone.length === LEGACY_SRS_IDS.length, srsRowsDone.join(', ')),
    check('todo_legacy_rows_closed', openLegacyRows.length === 0, openLegacyRows.join(', ')),
  ];

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'knowledge_graph_query_acceleration_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: LEGACY_SRS_IDS,
    generated_at: new Date().toISOString(),
    gate_id: GATE_ID,
    artifacts: { json: outJson, markdown: outMarkdown },
    source_contracts: {
      state: STATE_SOURCE,
      types: TYPES_SOURCE,
      paths: PATHS_SOURCE,
      query: QUERY_SOURCE,
      runtime: RUNTIME_SOURCE,
      subsystem: SUBSYSTEM_SOURCE,
    },
    regression_contracts: [REGRESSION_SOURCE],
    checks,
  };

  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);

  if (!pass) {
    const failed = checks.filter((row) => !row.ok).map((row) => row.id).join(', ');
    console.error(`knowledge graph query acceleration closure guard failed: ${failed}`);
    if (strict) process.exitCode = 1;
    return;
  }
  console.log(`knowledge graph query acceleration closure guard passed: ${outJson}`);
}

main();
