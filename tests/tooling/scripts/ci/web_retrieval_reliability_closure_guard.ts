#!/usr/bin/env tsx

import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'fs';
import { dirname, join } from 'path';

const SRS_ID = 'V12-WEB-RETRIEVAL-RELIABILITY-CLOSURE-001';
const LEGACY_SRS_IDS = [
  'V11-WEB-004',
  'V11-WEB-005',
  'V11-WEB-006',
  'V11-WEB-008',
  'V11-WEB-010',
  'V11-WEB-011',
  'V11-WEB-012',
  'V11-WEB-013',
  'V11-WEB-014',
];

const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PROFILES = 'tests/tooling/config/verify_profiles.json';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const PACKAGE_JSON = 'package.json';
const SRS = 'docs/workspace/SRS.md';
const TODO = 'docs/workspace/TODO.md';
const OUT_JSON = 'core/local/artifacts/web_retrieval_reliability_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/WEB_RETRIEVAL_RELIABILITY_CLOSURE_GUARD_CURRENT.md';
const GATE_ID = 'ops:web-retrieval:reliability-closure:guard';

const SOURCE_PATHS = {
  domain: 'core/layer0/ops/src/web_conduit_parts/020-domain-and-render.rs',
  fetch: 'core/layer0/ops/src/web_conduit_parts/031-fetch-transport-and-ssrf.rs',
  fetchApi: 'core/layer0/ops/src/web_conduit_parts/040-receipts-and-fetch-api.rs',
  serperBing: 'core/layer0/ops/src/web_conduit_parts/030-serper-bing-and-fetch.rs',
  redirect: 'core/layer0/ops/src/web_conduit_parts/025-fetch-utils-and-redirect.rs',
  pdfRuntime: 'core/layer0/ops/src/web_conduit_parts/084-pdf-tool-runtime.rs',
  pdfCli: 'core/layer0/ops/src/web_conduit_parts/083-cli-document-commands.rs',
  mediaHelpers: 'core/layer0/ops/src/web_conduit_parts/068-media-helpers.rs',
  providerRuntime: 'core/layer0/ops/src/web_conduit_provider_runtime.rs',
  webPolicy: 'client/runtime/config/web_conduit_policy.json',
  batchQuery: 'core/layer0/ops/src/batch_query_primitive_parts',
};

type Check = { id: string; ok: boolean; detail?: string };

function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  return process.argv.find((item) => item.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function flag(name: string, fallback: boolean): boolean {
  const value = arg(name, fallback ? '1' : '0').toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function readText(path: string): string {
  return readFileSync(path, 'utf8');
}

function readJson(path: string): any {
  return JSON.parse(readText(path));
}

function readTree(path: string): string {
  if (!existsSync(path)) return '';
  const stat = statSync(path);
  if (stat.isFile()) return readText(path);
  return readdirSync(path)
    .filter((entry) => !entry.startsWith('.'))
    .map((entry) => join(path, entry))
    .map((child) => readTree(child))
    .join('\n');
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

function includesAll(source: string, tokens: string[]): boolean {
  return missingTokens(source, tokens).length === 0;
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

function packageScript(pkg: any, name: string): string {
  const value = pkg?.scripts?.[name];
  return typeof value === 'string' ? value : '';
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

function doneRowsFor(content: string, id: string): number {
  const escaped = id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const tablePattern = new RegExp(`\\|\\s*${escaped}\\s*\\|\\s*done\\s*\\|`, 'g');
  const todoPattern = new RegExp('- \\\\[x\\\\] `' + escaped + '`', 'g');
  return (content.match(tablePattern) ?? []).length + (content.match(todoPattern) ?? []).length;
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Web Retrieval Reliability Closure Guard',
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
  const srs = readText(SRS);
  const todo = readText(TODO);
  const domain = readText(SOURCE_PATHS.domain);
  const fetch = readText(SOURCE_PATHS.fetch);
  const fetchApi = readText(SOURCE_PATHS.fetchApi);
  const serperBing = readText(SOURCE_PATHS.serperBing);
  const redirect = readText(SOURCE_PATHS.redirect);
  const pdfRuntime = readText(SOURCE_PATHS.pdfRuntime);
  const pdfCli = readText(SOURCE_PATHS.pdfCli);
  const mediaHelpers = readText(SOURCE_PATHS.mediaHelpers);
  const providerRuntime = readText(SOURCE_PATHS.providerRuntime);
  const policy = readText(SOURCE_PATHS.webPolicy);
  const webParts = readTree('core/layer0/ops/src/web_conduit_parts');
  const batchQuery = readTree(SOURCE_PATHS.batchQuery);
  const allWeb = [webParts, providerRuntime, policy].join('\n');
  const required = requiredArtifacts(manifest);
  const workload = workloadArtifacts(manifest);
  const reports = optionalReports(manifest);
  const artifacts = registryArtifacts(registry, GATE_ID);

  const checks: Check[] = [
    check('package_script_present', packageScript(pkg, GATE_ID).includes('tooling:run'), packageScript(pkg, GATE_ID)),
    check('registry_gate_runnable', registryRunnable(registry, GATE_ID), GATE_ID),
    check('registry_artifacts_declared', artifacts.includes(outJson) && artifacts.includes(outMarkdown), artifacts.join(', ')),
    check('fast_profile_covered', profileGateIds(profiles, 'fast').includes(GATE_ID), GATE_ID),
    check('boundary_profile_covered', profileGateIds(profiles, 'boundary').includes(GATE_ID), GATE_ID),
    check('release_profile_covered', profileGateIds(profiles, 'release').includes(GATE_ID), GATE_ID),
    check('required_artifact_declared', required.includes(outJson), outJson),
    check('workload_artifact_grouped', workload.includes(outJson), outJson),
    check('operator_report_declared', reports.includes(outMarkdown), outMarkdown),
    check('v11_web_rows_marked_done', LEGACY_SRS_IDS.every((id) => doneRowsFor(`${srs}\n${todo}`, id) > 0), LEGACY_SRS_IDS.map((id) => `${id}:${doneRowsFor(`${srs}\n${todo}`, id)}`).join(', ')),
    check('domain_scope_contract_present', includesAll(domain, ['normalize_allowed_domains', 'scoped_search_query', 'exclude_subdomains', '-site:*.', 'domain_matches_filter']), missingTokens(domain, ['normalize_allowed_domains', 'scoped_search_query', 'exclude_subdomains', '-site:*.', 'domain_matches_filter']).join(', ')),
    check('structured_provider_contract_present', includesAll(allWeb, ['serperdev', 'api_key_env', 'bing_rss', 'search_provider_order']), missingTokens(allWeb, ['serperdev', 'api_key_env', 'bing_rss', 'search_provider_order']).join(', ')),
    check('fetch_resilience_contract_present', includesAll(`${fetch}\n${fetchApi}`, ['fetch_serper_with_retry', 'Accept-Language', 'timeout_ms', 'max_response_bytes']), missingTokens(`${fetch}\n${fetchApi}`, ['fetch_serper_with_retry', 'Accept-Language', 'timeout_ms', 'max_response_bytes']).join(', ')),
    check('provider_failure_diagnostics_present', includesAll(serperBing, ['search_bing_fallback_reason', 'provider_errors', 'challenge', 'low_signal']), missingTokens(serperBing, ['search_bing_fallback_reason', 'provider_errors', 'challenge', 'low_signal']).join(', ')),
    check('provider_runtime_cache_catalog_present', includesAll(allWeb, ['provider_catalog_snapshot', 'fetch_provider_catalog_snapshot', 'default_fetch_provider_chain', 'cache_status']), missingTokens(allWeb, ['provider_catalog_snapshot', 'fetch_provider_catalog_snapshot', 'default_fetch_provider_chain', 'cache_status']).join(', ')),
    check('fetch_cache_extract_redirect_contract_present', includesAll(allWeb, ['normalize_search_result_link', 'extract_mode', 'resolved_url', 'cache_status']), missingTokens(allWeb, ['normalize_search_result_link', 'extract_mode', 'resolved_url', 'cache_status']).join(', ')),
    check('pdf_tool_contract_present', includesAll(`${pdfRuntime}\n${pdfCli}\n${mediaHelpers}`, ['web_media_pdf_tool_contract', 'api_pdf_tool', 'pdf_tool_default_model_preferences', 'pdf-tool']), missingTokens(`${pdfRuntime}\n${pdfCli}\n${mediaHelpers}`, ['web_media_pdf_tool_contract', 'api_pdf_tool', 'pdf_tool_default_model_preferences', 'pdf-tool']).join(', ')),
    check('batch_query_deictic_entity_gate_present', includesAll(batchQuery, ['resolve_deictic_framework_reference', 'comparison_entities_from_query', 'candidate_mentions_entity', 'coverage_ok']), missingTokens(batchQuery, ['resolve_deictic_framework_reference', 'comparison_entities_from_query', 'candidate_mentions_entity', 'coverage_ok']).join(', ')),
    check('batch_query_relevance_noise_gate_present', includesAll(batchQuery, ['candidate_passes_relevance_gate', 'portal', 'login', 'news', 'no_results']), missingTokens(batchQuery, ['candidate_passes_relevance_gate', 'portal', 'login', 'news', 'no_results']).join(', ')),
    check('batch_query_replay_cache_present', includesAll(batchQuery, ['cache_identity_query_plan', 'cache_status', 'hit', 'miss']), missingTokens(batchQuery, ['cache_identity_query_plan', 'cache_status', 'hit', 'miss']).join(', ')),
    check('batch_query_locator_hygiene_present', includesAll(batchQuery, ['candidate_from_search_payload', 'links', 'requested_url', 'search_payload_prefers_result_link_locator_over_search_engine_request_url']), missingTokens(batchQuery, ['candidate_from_search_payload', 'links', 'requested_url', 'search_payload_prefers_result_link_locator_over_search_engine_request_url']).join(', ')),
    check('batch_query_staged_orchestrator_present', includesAll(batchQuery, ['bing_rss', 'duckduckgo_instant', 'fixture_missing', 'minimum_synthesis_score']), missingTokens(batchQuery, ['bing_rss', 'duckduckgo_instant', 'fixture_missing', 'minimum_synthesis_score']).join(', ')),
    check('web_tooling_reliability_artifacts_required', required.includes('artifacts/web_tooling_context_soak_report_latest.json') && required.includes('artifacts/web_tooling_reliability_latest.json') && required.includes('core/local/artifacts/web_tooling_reliability_current.json'), 'web tooling artifacts'),
  ];

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'web_retrieval_reliability_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: LEGACY_SRS_IDS,
    generated_at: new Date().toISOString(),
    inputs: { registry_path: registryPath, profiles_path: profilesPath, manifest_path: manifestPath },
    summary: {
      checks: checks.length,
      passed: checks.filter((row) => row.ok).length,
      failed: checks.filter((row) => !row.ok).length,
    },
    checks,
    artifact_paths: [outJson, outMarkdown],
  };

  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  if (strict && !pass) process.exitCode = 1;
}

main();
