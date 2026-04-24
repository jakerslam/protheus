#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

type ProviderPolicy = {
  provider_id: string;
  typed_capability: string;
  required_probe_key: string;
  mode: 'search' | 'fetch';
  endpoint_env: string;
  api_key_envs?: string[];
  query_param?: string;
  url_param?: string;
  sample_query?: string;
  sample_url?: string;
};

type Policy = {
  version: number;
  providers: Record<string, ProviderPolicy>;
  bootstrap_contract?: {
    required_env?: string[];
    allow_skip_without_credentials?: boolean;
    required_skip_reasons?: string[];
  };
  rate_limit_contract?: {
    per_provider_burst_max?: number;
    sustained_rps_max?: number;
    on_budget_exhausted?: string;
  };
  circuit_breaker_contract?: {
    states?: string[];
    quarantine_transition_required?: boolean;
    cooldown_seconds_min?: number;
  };
  cache_contract?: {
    skip_reason_required_when_skipped?: boolean;
    stale_age_seconds_field_required?: boolean;
    write_block_on_provider_failure?: boolean;
  };
  diagnostics_contract?: {
    required_fields?: string[];
  };
  provider_failure_reason_codes?: string[];
};

type ProviderRun = {
  provider_id: string;
  mode: 'search' | 'fetch';
  typed_capability: string;
  required_probe_key: string;
  endpoint_env: string;
  endpoint_present: boolean;
  credentials_present: boolean;
  skipped: boolean;
  skip_reason: string;
  ran: boolean;
  ok: boolean;
  status_code: number;
  duration_ms: number;
  route_failure_reason: string;
  probe_failure_reason: string;
  provider_failure_reason: string;
  next_fix_hint: string;
};

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/gem_feedback_closure_policy.json');

function cleanText(value: unknown, maxLen = 240): string {
  return String(value ?? '').trim().replace(/\s+/g, ' ').slice(0, maxLen);
}

function parseBool(value: string | undefined, fallback = false): boolean {
  const normalized = String(value || '').trim().toLowerCase();
  if (!normalized) return fallback;
  return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on';
}

function parseArgs(argv: string[]) {
  let strict = false;
  let out = 'core/local/artifacts/gem_live_provider_smoke_current.json';
  let outMarkdown = 'local/workspace/reports/GEM_LIVE_PROVIDER_SMOKE_CURRENT.md';
  let timeoutMs = Math.max(
    2_000,
    Number.parseInt(process.env.INFRING_GEM_PROVIDER_SMOKE_TIMEOUT_MS || '8000', 10) || 8_000,
  );
  for (const token of argv) {
    if (token === '--strict') strict = true;
    else if (token.startsWith('--strict=')) strict = parseBool(token.slice('--strict='.length), false);
    else if (token.startsWith('--out=')) out = token.slice('--out='.length);
    else if (token.startsWith('--out-markdown=')) outMarkdown = token.slice('--out-markdown='.length);
    else if (token.startsWith('--timeout-ms=')) {
      const parsed = Number.parseInt(token.slice('--timeout-ms='.length), 10);
      if (Number.isFinite(parsed) && parsed >= 2_000) timeoutMs = parsed;
    }
  }
  return { strict, out, outMarkdown, timeoutMs };
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function writeJson(filePath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function writeMarkdown(filePath: string, body: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, body, 'utf8');
}

function resolveHeaderApiKey(apiKeyEnvs: string[] | undefined): { present: boolean; value: string } {
  const keys = Array.isArray(apiKeyEnvs) ? apiKeyEnvs : [];
  for (const key of keys) {
    const value = cleanText(process.env[key], 4096);
    if (value) return { present: true, value };
  }
  return { present: keys.length === 0, value: '' };
}

function buildProviderUrl(provider: ProviderPolicy): { url: string; valid: boolean } {
  const endpoint = cleanText(process.env[provider.endpoint_env], 4096);
  if (!endpoint) return { url: '', valid: false };
  if (!/^https?:\/\//i.test(endpoint)) return { url: endpoint, valid: false };
  const url = new URL(endpoint);
  if (provider.mode === 'search') {
    const query = cleanText(provider.sample_query || 'infring runtime', 160) || 'infring runtime';
    if (endpoint.includes('{query}')) {
      return { url: endpoint.replace('{query}', encodeURIComponent(query)), valid: true };
    }
    url.searchParams.set(cleanText(provider.query_param || 'q', 40) || 'q', query);
    return { url: url.toString(), valid: true };
  }
  const sampleUrl = cleanText(provider.sample_url || 'https://example.com/', 600);
  if (endpoint.includes('{url}')) {
    return { url: endpoint.replace('{url}', encodeURIComponent(sampleUrl)), valid: true };
  }
  url.searchParams.set(cleanText(provider.url_param || 'url', 40) || 'url', sampleUrl);
  return { url: url.toString(), valid: true };
}

async function fetchWithTimeout(
  url: string,
  timeoutMs: number,
  apiKey: string,
): Promise<{ ok: boolean; statusCode: number; bodyLength: number; failureReason: string }> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const headers: Record<string, string> = { Accept: 'application/json,text/plain,text/html' };
    if (apiKey) headers.Authorization = `Bearer ${apiKey}`;
    const response = await fetch(url, { method: 'GET', headers, signal: controller.signal });
    const body = await response.text();
    const bodyLength = body.length;
    const ok = response.ok && bodyLength > 0;
    return {
      ok,
      statusCode: response.status,
      bodyLength,
      failureReason: ok ? '' : response.status === 429 ? 'provider_rate_limited' : 'provider_unreachable',
    };
  } catch (error) {
    const text = String((error as Error)?.message || '').toLowerCase();
    const timeout = text.includes('abort') || text.includes('timed out');
    return {
      ok: false,
      statusCode: 0,
      bodyLength: 0,
      failureReason: timeout ? 'provider_timeout' : 'provider_unreachable',
    };
  } finally {
    clearTimeout(timer);
  }
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# GEM Live Provider Smoke (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at, 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- run_mode: ${cleanText(report.run_mode, 80)}`);
  lines.push('');
  lines.push('## Checks');
  for (const check of Array.isArray(report.checks) ? report.checks : []) {
    lines.push(`- ${check.id}: ${check.ok === true ? 'pass' : 'fail'} (${cleanText(check.detail, 200)})`);
  }
  lines.push('');
  lines.push('## Providers');
  for (const row of Array.isArray(report.providers) ? report.providers : []) {
    lines.push(
      `- ${cleanText(row.provider_id, 120)}: ok=${row.ok === true}; skipped=${row.skipped === true}; reason=${cleanText(row.skip_reason || row.provider_failure_reason || 'none', 200)}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

async function main(): Promise<number> {
  const args = parseArgs(process.argv.slice(2));
  const policy = readJson<Policy>(POLICY_PATH, {
    version: 0,
    providers: {},
  });
  const checks: Check[] = [];
  const requiredSkipReasons = new Set(
    Array.isArray(policy.bootstrap_contract?.required_skip_reasons)
      ? policy.bootstrap_contract?.required_skip_reasons || []
      : [],
  );
  const allowSkipWithoutCredentials = policy.bootstrap_contract?.allow_skip_without_credentials !== false;
  const providers: ProviderRun[] = [];

  const requiredEnv = Array.isArray(policy.bootstrap_contract?.required_env)
    ? policy.bootstrap_contract?.required_env || []
    : [];
  const missingRequiredEnv = requiredEnv.filter((key) => cleanText(process.env[key], 4096).length === 0);

  checks.push({
    id: 'gem_bootstrap_required_env_contract',
    ok: missingRequiredEnv.length === 0 || allowSkipWithoutCredentials,
    detail:
      missingRequiredEnv.length === 0
        ? 'all required provider env keys are present'
        : `missing=${missingRequiredEnv.join(',')};allow_skip_without_credentials=${allowSkipWithoutCredentials}`,
  });

  const providerRows = Object.values(policy.providers || {});
  checks.push({
    id: 'gem_provider_contract_rows_present',
    ok: providerRows.length >= 2,
    detail: `provider_rows=${providerRows.length}`,
  });

  let executedCount = 0;
  let skippedCount = 0;

  for (const row of providerRows) {
    const startedMs = Date.now();
    const endpointRaw = cleanText(process.env[row.endpoint_env], 4096);
    const endpointPresent = endpointRaw.length > 0;
    const keyResolution = resolveHeaderApiKey(row.api_key_envs);
    const credentialsPresent = keyResolution.present;
    const typedCapabilityOk =
      (row.typed_capability === 'web_search' || row.typed_capability === 'web_fetch') &&
      row.required_probe_key === row.typed_capability;
    checks.push({
      id: `gem_provider_typed_contract:${row.provider_id}`,
      ok: typedCapabilityOk,
      detail: `typed_capability=${row.typed_capability};required_probe_key=${row.required_probe_key}`,
    });

    const providerRun: ProviderRun = {
      provider_id: row.provider_id,
      mode: row.mode,
      typed_capability: row.typed_capability,
      required_probe_key: row.required_probe_key,
      endpoint_env: row.endpoint_env,
      endpoint_present: endpointPresent,
      credentials_present: credentialsPresent,
      skipped: false,
      skip_reason: '',
      ran: false,
      ok: false,
      status_code: 0,
      duration_ms: 0,
      route_failure_reason: '',
      probe_failure_reason: '',
      provider_failure_reason: '',
      next_fix_hint: '',
    };

    if (!endpointPresent) {
      providerRun.skipped = true;
      providerRun.skip_reason = 'provider_endpoint_missing';
      providerRun.route_failure_reason = 'route_provider_endpoint_missing';
      providerRun.probe_failure_reason = 'typed_probe_contract_missing_provider_endpoint';
      providerRun.provider_failure_reason = 'provider_registry_missing';
      providerRun.next_fix_hint = `set ${row.endpoint_env} to a reachable provider endpoint`;
      providerRun.ok = allowSkipWithoutCredentials;
      skippedCount += 1;
      providers.push(providerRun);
      continue;
    }

    if (!/^https?:\/\//i.test(endpointRaw)) {
      providerRun.skipped = true;
      providerRun.skip_reason = 'provider_invalid_configuration';
      providerRun.route_failure_reason = 'route_provider_endpoint_invalid';
      providerRun.probe_failure_reason = 'typed_probe_contract_provider_endpoint_invalid';
      providerRun.provider_failure_reason = 'provider_invalid_configuration';
      providerRun.next_fix_hint = `ensure ${row.endpoint_env} begins with http:// or https://`;
      providerRun.ok = false;
      skippedCount += 1;
      providers.push(providerRun);
      continue;
    }

    if (!credentialsPresent) {
      providerRun.skipped = true;
      providerRun.skip_reason = 'provider_credentials_missing';
      providerRun.route_failure_reason = 'route_provider_credentials_missing';
      providerRun.probe_failure_reason = 'typed_probe_contract_missing_provider_credentials';
      providerRun.provider_failure_reason = 'provider_auth_missing';
      providerRun.next_fix_hint = `set one of [${(row.api_key_envs || []).join(', ')}]`;
      providerRun.ok = allowSkipWithoutCredentials;
      skippedCount += 1;
      providers.push(providerRun);
      continue;
    }

    const builtUrl = buildProviderUrl(row);
    if (!builtUrl.valid || !builtUrl.url) {
      providerRun.skipped = true;
      providerRun.skip_reason = 'provider_invalid_configuration';
      providerRun.route_failure_reason = 'route_provider_request_shape_invalid';
      providerRun.probe_failure_reason = 'typed_probe_contract_provider_request_shape_invalid';
      providerRun.provider_failure_reason = 'provider_invalid_configuration';
      providerRun.next_fix_hint = 'verify provider endpoint template and sample query/url parameters';
      providerRun.ok = false;
      skippedCount += 1;
      providers.push(providerRun);
      continue;
    }

    providerRun.ran = true;
    executedCount += 1;
    const fetched = await fetchWithTimeout(builtUrl.url, args.timeoutMs, keyResolution.value);
    providerRun.status_code = fetched.statusCode;
    providerRun.duration_ms = Date.now() - startedMs;
    providerRun.ok = fetched.ok;
    providerRun.provider_failure_reason = cleanText(fetched.failureReason, 120);
    providerRun.route_failure_reason = fetched.ok ? '' : 'route_provider_call_failed';
    providerRun.probe_failure_reason = fetched.ok ? '' : 'typed_probe_provider_transport_unavailable';
    providerRun.next_fix_hint = fetched.ok
      ? 'none'
      : fetched.failureReason === 'provider_rate_limited'
      ? 'reduce call rate and verify rate-limit budget policy'
      : fetched.failureReason === 'provider_timeout'
      ? 'verify provider reachability and increase timeout or network stability'
      : 'verify provider endpoint and credentials';
    providers.push(providerRun);
  }

  const diagnosticsRequiredFields = Array.isArray(policy.diagnostics_contract?.required_fields)
    ? policy.diagnostics_contract?.required_fields || []
    : [];
  const diagnosticRows = providers.filter((row) => !row.ok);
  const diagnosticsContractOk = diagnosticRows.every((row) =>
    diagnosticsRequiredFields.every((field) => cleanText((row as any)[field], 200).length > 0),
  );
  checks.push({
    id: 'gem_diagnostics_contract_required_fields',
    ok: diagnosticsContractOk,
    detail: `failed_provider_rows=${diagnosticRows.length};required_fields=${diagnosticsRequiredFields.join(',')}`,
  });
  const requiredReasonCodes = new Set(
    Array.isArray(policy.provider_failure_reason_codes)
      ? policy.provider_failure_reason_codes.map((row) => cleanText(row, 120)).filter(Boolean)
      : [],
  );
  const providerReasonCodesOk = diagnosticRows.every((row) => {
    const reason = cleanText(row.provider_failure_reason, 120);
    return reason.length > 0 && (requiredReasonCodes.size === 0 || requiredReasonCodes.has(reason));
  });
  checks.push({
    id: 'gem_provider_failure_reason_codes_canonical',
    ok: providerReasonCodesOk,
    detail:
      requiredReasonCodes.size === 0
        ? 'no provider failure reason allowlist configured'
        : `required_reason_codes=${Array.from(requiredReasonCodes).join(',')}`,
  });

  const skipReasonsValid = providers
    .filter((row) => row.skipped)
    .every((row) => requiredSkipReasons.size === 0 || requiredSkipReasons.has(row.skip_reason));
  checks.push({
    id: 'gem_provider_skip_reason_contract',
    ok: skipReasonsValid,
    detail:
      requiredSkipReasons.size === 0
        ? 'no required skip reason allowlist configured'
        : `required_skip_reasons=${Array.from(requiredSkipReasons).join(',')}`,
  });

  const rate = policy.rate_limit_contract || {};
  checks.push({
    id: 'gem_rate_limit_contract',
    ok:
      Number(rate.per_provider_burst_max) > 0 &&
      Number(rate.sustained_rps_max) > 0 &&
      cleanText(rate.on_budget_exhausted, 120) === 'fail_closed',
    detail: `burst_max=${cleanText(rate.per_provider_burst_max, 40)};sustained_rps_max=${cleanText(
      rate.sustained_rps_max,
      40,
    )};on_budget_exhausted=${cleanText(rate.on_budget_exhausted, 120)}`,
  });

  const breaker = policy.circuit_breaker_contract || {};
  const states = Array.isArray(breaker.states) ? breaker.states.map((row) => cleanText(row, 40)) : [];
  checks.push({
    id: 'gem_circuit_breaker_contract',
    ok:
      states.includes('closed') &&
      states.includes('half_open') &&
      states.includes('open') &&
      breaker.quarantine_transition_required === true &&
      Number(breaker.cooldown_seconds_min) > 0,
    detail: `states=${states.join('|')};quarantine_transition_required=${breaker.quarantine_transition_required === true};cooldown_seconds_min=${cleanText(
      breaker.cooldown_seconds_min,
      40,
    )}`,
  });

  const cache = policy.cache_contract || {};
  checks.push({
    id: 'gem_cache_contract',
    ok:
      cache.skip_reason_required_when_skipped === true &&
      cache.stale_age_seconds_field_required === true &&
      cache.write_block_on_provider_failure === true,
    detail: `skip_reason_required_when_skipped=${cache.skip_reason_required_when_skipped === true};stale_age_seconds_field_required=${cache.stale_age_seconds_field_required === true};write_block_on_provider_failure=${cache.write_block_on_provider_failure === true}`,
  });

  checks.push({
    id: 'gem_provider_live_smoke_or_skip_contract',
    ok: providers.length > 0 && providers.every((row) => row.ran || row.skipped),
    detail: `providers=${providers.length};executed=${executedCount};skipped=${skippedCount}`,
  });

  const failed = checks.filter((row) => !row.ok);
  const ok = failed.length === 0;
  const runMode =
    executedCount > 0
      ? 'executed'
      : skippedCount > 0
      ? 'skipped_credentials'
      : 'no_provider_rows';

  const report = {
    type: 'gem_live_provider_smoke',
    schema_version: 1,
    generated_at: new Date().toISOString(),
    strict: args.strict,
    ok,
    run_mode: runMode,
    summary: {
      provider_count: providers.length,
      executed_count: executedCount,
      skipped_count: skippedCount,
      failed_check_count: failed.length,
      pass: ok,
    },
    policy_path: path.relative(ROOT, POLICY_PATH),
    providers,
    checks,
    failed_ids: failed.map((row) => row.id),
  };

  const outPath = path.resolve(ROOT, args.out);
  const outLatestPath = path.resolve(ROOT, 'artifacts/gem_live_provider_smoke_latest.json');
  const outStatePath = path.resolve(ROOT, 'local/state/ops/gem_live_provider_smoke/latest.json');
  const markdownPath = path.resolve(ROOT, args.outMarkdown);
  writeJson(outPath, report);
  writeJson(outLatestPath, report);
  writeJson(outStatePath, report);
  writeMarkdown(markdownPath, renderMarkdown(report));
  process.stdout.write(`${JSON.stringify(report)}\n`);
  if (args.strict && !ok) return 1;
  return 0;
}

main()
  .then((code) => process.exit(code))
  .catch((error) => {
    const outPath = path.resolve(
      ROOT,
      'core/local/artifacts/gem_live_provider_smoke_current.json',
    );
    writeJson(outPath, {
      type: 'gem_live_provider_smoke',
      schema_version: 1,
      generated_at: new Date().toISOString(),
      ok: false,
      error: cleanText((error as Error)?.message || 'gem_live_provider_smoke_failed', 800),
    });
    process.exit(1);
  });
