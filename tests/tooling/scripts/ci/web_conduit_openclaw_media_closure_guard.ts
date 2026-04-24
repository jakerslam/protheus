#!/usr/bin/env tsx

import { mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-WEB-CONDUIT-OPENCLAW-MEDIA-CLOSURE-001';
const LEGACY_SRS_IDS = [
  'V10-WEB-CONDUIT-001.12',
  'V10-WEB-CONDUIT-001.13',
  'V10-WEB-CONDUIT-001.14',
  'V10-WEB-CONDUIT-001.15',
  'V10-WEB-CONDUIT-001.16',
  'V10-WEB-CONDUIT-001.17',
  'V10-WEB-CONDUIT-001.18',
  'V10-WEB-CONDUIT-001.19',
  'V10-WEB-CONDUIT-001.20',
  'V10-WEB-CONDUIT-001.21',
  'V10-WEB-CONDUIT-001.22',
  'V10-WEB-CONDUIT-001.23',
];
const GATE_ID = 'ops:web-conduit:openclaw-media:closure:guard';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PROFILES = 'tests/tooling/config/verify_profiles.json';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const PACKAGE_JSON = 'package.json';
const OUT_JSON = 'core/local/artifacts/web_conduit_openclaw_media_closure_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/WEB_CONDUIT_OPENCLAW_MEDIA_CLOSURE_GUARD_CURRENT.md';

const PROVIDER_CONTRACTS = 'core/layer0/ops/src/web_conduit_provider_runtime_parts/017-provider-public-contracts.rs';
const FETCH_TRANSPORT = 'core/layer0/ops/src/web_conduit_parts/031-fetch-transport-and-ssrf.rs';
const FETCH_VISIBILITY = 'core/layer0/ops/src/web_conduit_parts/026-fetch-visibility-and-readability.rs';
const MEDIA_PARSE = 'core/layer0/ops/src/web_conduit_parts/027-media-parse-and-base64.rs';
const FILE_CONTEXT = 'core/layer0/ops/src/web_conduit_parts/028-media-file-context-and-budgets.rs';
const OUTBOUND_AUDIO = 'core/layer0/ops/src/web_conduit_parts/029-media-outbound-and-audio.rs';
const MEDIA_HELPERS = 'core/layer0/ops/src/web_conduit_parts/068-media-helpers.rs';
const MEDIA_RUNTIME = 'core/layer0/ops/src/web_conduit_parts/069-media-runtime.rs';
const CLI_RUN = 'core/layer0/ops/src/web_conduit_parts/070-cli-run.rs';
const LOCAL_PATH_POLICY = 'core/layer0/ops/src/web_conduit_parts/071-media-local-path-policy.rs';
const MEDIA_HOSTING = 'core/layer0/ops/src/web_conduit_parts/072-media-hosting.rs';
const MEDIA_STORE = 'core/layer0/ops/src/web_conduit_parts/073-media-store.rs';
const DASHBOARD_MEDIA_ROUTE_B = 'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/late_b.rs';
const DASHBOARD_MEDIA_ROUTE_C = 'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/late_c.rs';
const TEST_PROVIDER_PROOF = 'core/layer0/ops/src/web_conduit_parts/097-openclaw-runtime-provider-proof-tests.rs';
const TEST_MEDIA = 'core/layer0/ops/src/web_conduit_parts/099-openclaw-media-tests.rs';
const TEST_FETCH = 'core/layer0/ops/src/web_conduit_parts/090-openclaw-fetch-helper-tests.rs';
const TEST_HOST = 'core/layer0/ops/src/web_conduit_parts/098-openclaw-media-host-tests.rs';
const TEST_OUTBOUND = 'core/layer0/ops/src/web_conduit_parts/100-openclaw-outbound-audio-tests.rs';
const TEST_STORE = 'core/layer0/ops/src/web_conduit_parts/101-openclaw-media-store-tests.rs';
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
    '# Web Conduit OpenClaw Media Closure Guard',
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
  const providerSources = readMany([PROVIDER_CONTRACTS, TEST_PROVIDER_PROOF]);
  const mediaSources = readMany([MEDIA_HELPERS, MEDIA_RUNTIME, CLI_RUN, LOCAL_PATH_POLICY, TEST_MEDIA]);
  const fetchSources = readMany([FETCH_TRANSPORT, FETCH_VISIBILITY, TEST_FETCH]);
  const parseAndContextSources = readMany([MEDIA_PARSE, FILE_CONTEXT, CLI_RUN]);
  const hostSources = readMany([MEDIA_HOSTING, DASHBOARD_MEDIA_ROUTE_B, DASHBOARD_MEDIA_ROUTE_C, TEST_HOST]);
  const outboundAndStoreSources = readMany([OUTBOUND_AUDIO, MEDIA_STORE, TEST_OUTBOUND, TEST_STORE]);
  const srs = readText(SRS);
  const todo = readText(TODO);
  const required = requiredArtifacts(manifest);
  const workload = workloadArtifacts(manifest);
  const reports = optionalReports(manifest);

  const providerTokens = [
    'supported_provider_ids',
    'unsupported_provider_examples',
    'openclaw_runtime_contract_search_runtime_prefers_keyless_fallback_without_credentials',
    'openclaw_runtime_contract_fetch_runtime_falls_back_to_direct_http_for_invalid_provider',
    'openclaw_runtime_contract_duckduckgo_search_contract_is_keyless_and_allowlisted',
    'openclaw_runtime_contract_xai_search_contract_fails_closed_outside_allowlist',
  ];
  const mediaTokens = [
    'media_request_contract',
    'managed_canvas_media_prefix',
    'default_local_root_suffixes',
    'supports_wildcard_local_roots',
    'channel_attachment_root_contract',
    'host_read_policy_contract',
    'invalid-root',
    'fetch_stalled',
    'application/octet-stream',
    'openclaw_media_remote_fetch_classifies_stalled_transfer',
    'openclaw_media_accepts_wildcard_local_roots_for_attachment_paths',
    'openclaw_media_disables_unbounded_host_reads_when_sender_policy_denies_read',
  ];
  const fetchTokens = [
    'FETCH_MARKDOWN_ACCEPT_HEADER',
    'evaluate_fetch_ssrf_guard',
    'text/markdown',
    'cf-markdown',
    'strip_invisible_unicode',
    'openclaw_fetch_ssrf_guard_blocks_private_targets_and_localhost',
  ];
  const parseAndContextTokens = [
    'parse-media',
    'MEDIA:',
    '[[audio_as_voice]]',
    'data:',
    'file-context',
    'media_kind_budget_contract',
    'file_context_contract',
    'web_media_file_context',
  ];
  const hostTokens = [
    'media-host',
    'media_host_contract',
    '/api/web/media',
    '/api/web/media-host',
    'openclaw_media_host_roundtrip_creates_delivery_route_and_cleans_up',
    'openclaw_media_host_expires_entries_fail_closed',
    'openclaw_media_host_read_rejects_outside_workspace_manifests',
  ];
  const outboundAndStoreTokens = [
    'web_conduit_outbound_attachment',
    'voice_compatible_audio',
    'is_telegram_voice_compatible_audio',
    'media_store_contract',
    'saved_id_shape',
    'openclaw_media_reports_voice_compatible_audio_for_m4a_extension',
    'openclaw_media_store_resolve_and_delete_are_fail_closed',
    'openclaw_media_store_cleanup_prunes_expired_files_and_empty_dirs',
  ];

  const missingProvider = missingTokens(providerSources, providerTokens);
  const missingMedia = missingTokens(mediaSources, mediaTokens);
  const missingFetch = missingTokens(fetchSources, fetchTokens);
  const missingParseContext = missingTokens(parseAndContextSources, parseAndContextTokens);
  const missingHost = missingTokens(hostSources, hostTokens);
  const missingOutboundStore = missingTokens(outboundAndStoreSources, outboundAndStoreTokens);
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
    check('provider_reference_proof_tokens_present', missingProvider.length === 0, missingProvider.join(', ')),
    check('media_fetch_path_policy_and_guard_tokens_present', missingMedia.length === 0, missingMedia.join(', ')),
    check('fetch_visibility_ssrf_markdown_tokens_present', missingFetch.length === 0, missingFetch.join(', ')),
    check('parse_inline_file_context_tokens_present', missingParseContext.length === 0, missingParseContext.join(', ')),
    check('hosted_delivery_tokens_present', missingHost.length === 0, missingHost.join(', ')),
    check('outbound_audio_store_tokens_present', missingOutboundStore.length === 0, missingOutboundStore.join(', ')),
    check('srs_closure_row_present', srs.includes(SRS_ID) && srs.includes(GATE_ID), SRS_ID),
    check('legacy_srs_rows_done', srsRowsDone.length === LEGACY_SRS_IDS.length, srsRowsDone.join(', ')),
    check('todo_legacy_rows_closed', openLegacyRows.length === 0, openLegacyRows.join(', ')),
  ];

  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'web_conduit_openclaw_media_closure_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: LEGACY_SRS_IDS,
    generated_at: new Date().toISOString(),
    gate_id: GATE_ID,
    artifacts: { json: outJson, markdown: outMarkdown },
    source_contracts: {
      provider: [PROVIDER_CONTRACTS, TEST_PROVIDER_PROOF],
      media: [MEDIA_HELPERS, MEDIA_RUNTIME, CLI_RUN, LOCAL_PATH_POLICY, TEST_MEDIA],
      fetch: [FETCH_TRANSPORT, FETCH_VISIBILITY, TEST_FETCH],
      parse_and_context: [MEDIA_PARSE, FILE_CONTEXT, CLI_RUN],
      hosting: [MEDIA_HOSTING, DASHBOARD_MEDIA_ROUTE_B, DASHBOARD_MEDIA_ROUTE_C, TEST_HOST],
      outbound_and_store: [OUTBOUND_AUDIO, MEDIA_STORE, TEST_OUTBOUND, TEST_STORE],
    },
    checks,
  };

  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);

  if (!pass) {
    const failed = checks.filter((row) => !row.ok).map((row) => row.id).join(', ');
    console.error(`web conduit OpenClaw media closure guard failed: ${failed}`);
    if (strict) process.exitCode = 1;
    return;
  }
  console.log(`web conduit OpenClaw media closure guard passed: ${outJson}`);
}

main();
