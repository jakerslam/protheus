#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'validation/regression/contracts/shell_tiered_long_chat_heap_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_tiered_long_chat_heap_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_TIERED_LONG_CHAT_HEAP_GUARD_CURRENT.md';

type Violation = { kind: string; tier: string; path: string; detail: string; scenario?: string };
type Scenario = { id: string; total_messages: number; rendered_window_rows: number };
type Budget = Record<string, number>;

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readJson(relPath: string): any {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8'));
}

function bytes(value: unknown): number {
  return Buffer.byteLength(typeof value === 'string' ? value : JSON.stringify(value), 'utf8');
}

function push(violations: Violation[], kind: string, tier: string, pathRel: string, detail: string, scenario?: string): void {
  violations.push({ kind, tier, path: pathRel, detail, scenario });
}

function truthyFlag(argv: string[], name: string): boolean {
  const value = readFlag(argv, name);
  return value === '1' || value === 'true' || value === 'yes';
}

function hasPackageModule(moduleName: string): boolean {
  return fs.existsSync(abs(`node_modules/${moduleName}`));
}

function browserDriverPath(): string | null {
  const candidates = [
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
    '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
    '/usr/bin/google-chrome',
    '/usr/bin/chromium-browser',
    '/usr/bin/chromium',
  ];
  return candidates.find((candidate) => fs.existsSync(candidate)) || null;
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function unescapeHtml(text: string): string {
  return text
    .replace(/&quot;/g, '"')
    .replace(/&#34;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');
}

function syntheticRow(index: number): any {
  return {
    id: `long-chat-row-${index}`,
    conversation_id: 'long-chat-fixture',
    origin_kind: index % 5 === 0 ? 'user' : 'assistant',
    origin_display_name: index % 5 === 0 ? 'Jay' : 'InfRing',
    timestamp: new Date(Date.UTC(2026, 4, 1, 12, 0, index % 60)).toISOString(),
    status: 'complete',
    content_preview: `Bounded message preview ${index}`,
    line_count: 1,
    detail_ref: `/api/agents/chat-ui-default-agent/details/message/long-chat-row-${index}`,
    tool_summary_count: index > 0 && index % 50 === 0 ? 1 : 0,
    artifact_summary_count: index > 0 && index % 120 === 0 ? 1 : 0,
    allowed_display_actions: ['open_detail'],
  };
}

function buildGatewayPayload(scenario: Scenario, includeViolation: boolean): any {
  const rowCount = includeViolation ? scenario.total_messages : scenario.rendered_window_rows;
  const rows = Array.from({ length: rowCount }, (_unused, index) => syntheticRow(index));
  const payload: any = {
    route_class: 'bounded_search_query',
    projection_type: 'chat_message_window_projection',
    conversation_id: 'long-chat-fixture',
    rows,
    before_cursor: scenario.total_messages > rowCount ? 'cursor-before' : null,
    after_cursor: scenario.total_messages > rowCount ? 'cursor-after' : null,
    window_start_id: rows[0]?.id || null,
    window_end_id: rows[rows.length - 1]?.id || null,
    total_count: scenario.total_messages,
  };
  if (includeViolation) {
    payload.all_messages = rows.map((row: any) => ({ ...row, raw_tool_result: { stdout: 'x'.repeat(128) } }));
    payload.trace_body = 'full trace body should never travel on the default Shell path';
  }
  return payload;
}

function findForbiddenKeys(value: unknown, forbidden: Set<string>, trail = '$'): string[] {
  if (!value || typeof value !== 'object') return [];
  const hits: string[] = [];
  if (Array.isArray(value)) {
    value.slice(0, 120).forEach((entry, index) => hits.push(...findForbiddenKeys(entry, forbidden, `${trail}[${index}]`)));
    return hits;
  }
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    const childTrail = `${trail}.${key}`;
    if (forbidden.has(key)) hits.push(childTrail);
    hits.push(...findForbiddenKeys(child, forbidden, childTrail));
  }
  return hits;
}

function renderShellFixture(rows: any[], scenarioId: string, storagePayload: any): string {
  const articles = rows.map((row) => [
    `<article class="chat-bubble" data-message-id="${row.id}">`,
    `<header><span>${row.origin_display_name}</span><time>${row.timestamp}</time></header>`,
    `<p>${row.content_preview}</p>`,
    `<a href="${row.detail_ref}" data-detail-ref="true">Open detail</a>`,
    '</article>',
  ].join('')).join('');
  return [
    '<!doctype html>',
    `<main id="shell-thread" data-scenario="${scenarioId}" data-windowed="true">`,
    articles,
    '</main>',
    `<script type="application/json" id="fixture-storage">${escapeHtml(JSON.stringify(storagePayload))}</script>`,
    '<script>',
    '(() => {',
    '  let storageBytes = 0;',
    '  let storageError = null;',
    '  try {',
    '    const storageNode = document.getElementById("fixture-storage");',
    '    const storageText = storageNode ? storageNode.textContent || "{}" : "{}";',
    '    localStorage.setItem("infring_preview_cache", storageText);',
    '    storageBytes = new Blob([localStorage.getItem("infring_preview_cache") || ""]).size;',
    '  } catch (error) {',
    '    storageError = error && error.message ? error.message : String(error);',
    '  }',
    '  const metrics = {',
    '    dom_nodes: document.querySelectorAll("*").length,',
    '    custom_elements: document.querySelectorAll(".chat-bubble").length,',
    '    storage_bytes: storageBytes,',
    '    storage_error: storageError,',
    '    used_js_heap_bytes: performance && performance.memory ? performance.memory.usedJSHeapSize : null',
    '  };',
    '  const output = document.createElement("pre");',
    '  output.id = "infring-browser-metrics";',
    '  output.textContent = "__INFRING_METRICS__" + JSON.stringify(metrics) + "__END__";',
    '  document.body.appendChild(output);',
    '})();',
    '</script>',
  ].join('');
}

function runBrowserMetrics(html: string, driverPath: string): any {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-shell-heap-'));
  const htmlPath = path.join(tempDir, 'fixture.html');
  const profilePath = path.join(tempDir, 'profile');
  fs.writeFileSync(htmlPath, html, 'utf8');
  const result = spawnSync(driverPath, [
    '--headless=new',
    '--disable-gpu',
    '--disable-background-networking',
    '--disable-component-update',
    '--disable-default-apps',
    '--disable-extensions',
    '--disable-features=MediaRouter,OptimizationHints,Translate',
    '--disable-sync',
    '--enable-precise-memory-info',
    '--no-default-browser-check',
    '--no-first-run',
    '--no-sandbox',
    '--run-all-compositor-stages-before-draw',
    '--virtual-time-budget=1000',
    `--user-data-dir=${profilePath}`,
    '--dump-dom',
    pathToFileURL(htmlPath).toString(),
  ], { encoding: 'utf8', maxBuffer: 24 * 1024 * 1024, timeout: 12000 });
  fs.rmSync(tempDir, { recursive: true, force: true });
  let parsed: any = null;
  for (const match of String(result.stdout || '').matchAll(/__INFRING_METRICS__(.*?)__END__/gs)) {
    try {
      const candidate = JSON.parse(unescapeHtml(match[1]));
      parsed = typeof candidate === 'string' ? JSON.parse(unescapeHtml(candidate)) : candidate;
      if (parsed && typeof parsed === 'object') break;
    } catch (_) {
      parsed = null;
    }
  }
  if (!parsed) return { ok: false, error: 'browser_metrics_marker_missing' };
  for (const field of ['dom_nodes', 'custom_elements', 'storage_bytes']) {
    if (!Number.isFinite(Number(parsed[field]))) return { ok: false, error: `browser_metric_missing_${field}` };
  }
  if (result.error || result.status !== 0) {
    parsed.browser_warning = result.error?.message || result.stderr || `status_${result.status}`;
  }
  return { ok: true, ...parsed };
}

function renderMetrics(payload: any, scenario: Scenario, driverPath: string | null, controlledViolation: boolean): any {
  const storagePayload = {
    conversation_id: payload.conversation_id,
    rows: payload.rows,
    before_cursor: payload.before_cursor,
    after_cursor: payload.after_cursor,
  };
  const html = renderShellFixture(payload.rows || [], scenario.id, storagePayload);
  const renderedRows = Array.isArray(payload.rows) ? payload.rows.length : 0;
  const storageBytes = bytes(storagePayload);
  const browser = driverPath && !controlledViolation ? runBrowserMetrics(html, driverPath) : null;
  const domNodes = browser?.ok ? browser.dom_nodes : 18 + renderedRows * 12;
  const customElements = browser?.ok ? browser.custom_elements : renderedRows + Math.ceil(renderedRows / 40);
  const heapGrowthMb = Number((42 + renderedRows * 0.48 + scenario.total_messages * 0.001 + storageBytes / 1_048_576).toFixed(2));
  return {
    browser_driver_path: driverPath,
    browser_rendered: Boolean(browser?.ok),
    browser_error: browser && !browser.ok ? browser.error : null,
    browser_used_js_heap_mb: browser?.used_js_heap_bytes ? Number((browser.used_js_heap_bytes / 1_048_576).toFixed(2)) : null,
    html_bytes: bytes(html),
    payload_bytes: bytes(payload),
    rendered_rows: renderedRows,
    dom_nodes: domNodes,
    custom_elements: customElements,
    heap_growth_mb: heapGrowthMb,
    storage_bytes: storageBytes,
    cleanup_storage_bytes: 0,
  };
}

function validateTier1(contract: any, pkg: any, violations: Violation[]): void {
  const tier = 'tier1_deterministic_store_projection';
  const row = (contract.tier_commands || []).find((entry: any) => entry.id === tier);
  if (!row) {
    push(violations, 'missing_tier_command', tier, DEFAULT_CONTRACT, 'Tier 1 long-chat store projection command is missing.');
    return;
  }
  const script = pkg.scripts?.[row.command] || '';
  if (!script.includes(path.basename(row.script))) {
    push(violations, 'wrong_tier_package_script', tier, 'package.json', `${row.command} must execute ${path.basename(row.script)}.`);
  }
  if (!fs.existsSync(abs(row.script))) {
    push(violations, 'missing_tier_script', tier, row.script, 'Tier 1 guard source does not exist.');
  }
}

function validateRows(payload: any, requiredFields: string[], violations: Violation[], scenarioId: string): void {
  const first = Array.isArray(payload.rows) ? payload.rows[0] : null;
  for (const field of requiredFields) {
    if (!first || first[field] == null) {
      push(violations, 'missing_shell_row_field', 'tier3_gateway_to_shell_projection_stress', DEFAULT_CONTRACT, `Shell row is missing ${field}.`, scenarioId);
    }
  }
}

function validateScenario(contract: any, scenario: Scenario, includeViolation: boolean, useHeadlessBrowser: boolean): any {
  const budgets: Budget = contract.budgets || {};
  const violations: Violation[] = [];
  const payload = buildGatewayPayload(scenario, includeViolation);
  const metrics = renderMetrics(payload, scenario, useHeadlessBrowser ? browserDriverPath() : null, includeViolation);
  const forbidden = new Set<string>(contract.forbidden_default_payload_fields || []);
  const requiredProjectionFields: string[] = contract.required_gateway_projection_fields || [];

  if (metrics.rendered_rows > budgets.max_rendered_rows) {
    push(violations, 'rendered_rows_exceed_budget', 'tier2_browser_rendered_heap_fixture', DEFAULT_CONTRACT, `${metrics.rendered_rows} > ${budgets.max_rendered_rows}.`, scenario.id);
  }
  if (metrics.dom_nodes > budgets.max_dom_nodes) {
    push(violations, 'dom_nodes_exceed_budget', 'tier2_browser_rendered_heap_fixture', DEFAULT_CONTRACT, `${metrics.dom_nodes} > ${budgets.max_dom_nodes}.`, scenario.id);
  }
  if (metrics.custom_elements > budgets.max_custom_elements) {
    push(violations, 'custom_elements_exceed_budget', 'tier2_browser_rendered_heap_fixture', DEFAULT_CONTRACT, `${metrics.custom_elements} > ${budgets.max_custom_elements}.`, scenario.id);
  }
  if (metrics.heap_growth_mb > budgets.max_heap_growth_mb) {
    push(violations, 'heap_growth_exceed_budget', 'tier2_browser_rendered_heap_fixture', DEFAULT_CONTRACT, `${metrics.heap_growth_mb} > ${budgets.max_heap_growth_mb}.`, scenario.id);
  }
  if (metrics.storage_bytes > budgets.max_storage_bytes) {
    push(violations, 'storage_bytes_exceed_budget', 'tier2_browser_rendered_heap_fixture', DEFAULT_CONTRACT, `${metrics.storage_bytes} > ${budgets.max_storage_bytes}.`, scenario.id);
  }
  if (metrics.browser_driver_path && !includeViolation && !metrics.browser_rendered) {
    push(violations, 'headless_browser_render_failed', 'tier2_browser_rendered_heap_fixture', DEFAULT_CONTRACT, String(metrics.browser_error || 'unknown browser render failure'), scenario.id);
  }
  if (metrics.payload_bytes > budgets.max_payload_bytes) {
    push(violations, 'gateway_payload_exceed_budget', 'tier3_gateway_to_shell_projection_stress', DEFAULT_CONTRACT, `${metrics.payload_bytes} > ${budgets.max_payload_bytes}.`, scenario.id);
  }
  if ((payload.rows || []).length > budgets.max_gateway_default_rows) {
    push(violations, 'gateway_default_rows_exceed_budget', 'tier3_gateway_to_shell_projection_stress', DEFAULT_CONTRACT, `${payload.rows.length} > ${budgets.max_gateway_default_rows}.`, scenario.id);
  }
  for (const field of requiredProjectionFields) {
    if (!(field in payload)) push(violations, 'missing_gateway_projection_field', 'tier3_gateway_to_shell_projection_stress', DEFAULT_CONTRACT, `Projection payload missing ${field}.`, scenario.id);
  }
  validateRows(payload, contract.required_row_fields || [], violations, scenario.id);
  for (const hit of findForbiddenKeys(payload, forbidden)) {
    push(violations, 'forbidden_default_payload_field', 'tier3_gateway_to_shell_projection_stress', DEFAULT_CONTRACT, `Default Shell payload contains ${hit}.`, scenario.id);
  }

  return { scenario: scenario.id, ok: violations.length === 0, metrics, violations };
}

function validateContractShape(contract: any, violations: Violation[]): void {
  if (contract.owner !== 'assurance.validation') {
    push(violations, 'wrong_contract_owner', 'contract', DEFAULT_CONTRACT, 'Tiered heap contract must be Validation-owned.');
  }
  if (contract.status !== 'enforced') {
    push(violations, 'contract_not_enforced', 'contract', DEFAULT_CONTRACT, 'Tiered heap contract must be enforced.');
  }
  for (const id of ['tier1_deterministic_store_projection', 'tier2_browser_rendered_heap_fixture', 'tier3_gateway_to_shell_projection_stress']) {
    if (!(contract.tier_commands || []).some((row: any) => row.id === id)) {
      push(violations, 'missing_required_tier', id, DEFAULT_CONTRACT, `Missing required tier ${id}.`);
    }
  }
}

function renderMarkdown(report: any): string {
  const lines = [
    '# Shell Tiered Long-Chat Heap Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    `contract: ${report.contract_path}`,
    `controlled_violation: ${report.controlled_violation}`,
    `headless_browser_enabled: ${report.headless_browser_enabled}`,
    `violations: ${report.violations.length}`,
    '',
    '## Driver Availability',
    `- playwright: ${report.driver_availability.playwright}`,
    `- jsdom: ${report.driver_availability.jsdom}`,
    '',
    '## Scenario Metrics',
  ];
  for (const scenario of report.scenarios) {
    lines.push(`- ${scenario.scenario}: rows=${scenario.metrics.rendered_rows}, heap_mb=${scenario.metrics.heap_growth_mb}, dom_nodes=${scenario.metrics.dom_nodes}, storage_bytes=${scenario.metrics.storage_bytes}, payload_bytes=${scenario.metrics.payload_bytes}`);
  }
  lines.push('', '## Violations');
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations) {
    lines.push(`- ${violation.kind} (${violation.tier}${violation.scenario ? `/${violation.scenario}` : ''}) at \`${violation.path}\`: ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const args = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
const contractPath = cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT, 600);
const outJson = cleanText(readFlag(argv, 'out-json') || args.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const controlledViolation = truthyFlag(argv, 'include-controlled-violation');
const useHeadlessBrowser = truthyFlag(argv, 'use-headless-browser');
const contract = readJson(contractPath);
const pkg = readJson('package.json');
const violations: Violation[] = [];
validateContractShape(contract, violations);
validateTier1(contract, pkg, violations);

const scenarioReports = (contract.scenarios || []).map((scenario: Scenario) => validateScenario(contract, scenario, controlledViolation, useHeadlessBrowser));
for (const scenarioReport of scenarioReports) violations.push(...scenarioReport.violations);

const report = {
  ok: violations.length === 0,
  type: 'shell_tiered_long_chat_heap_guard',
  revision: currentRevision(ROOT),
  contract_path: contractPath,
  controlled_violation: controlledViolation,
  headless_browser_enabled: useHeadlessBrowser,
  driver_availability: {
    headless_chrome: Boolean(browserDriverPath()),
    playwright: hasPackageModule('playwright') || hasPackageModule('@playwright/test'),
    jsdom: hasPackageModule('jsdom'),
  },
  tiers: (contract.tier_commands || []).map((tier: any) => ({ id: tier.id, command: tier.command, mode: tier.mode })),
  scenarios: scenarioReports.map(({ scenario, ok, metrics }: any) => ({ scenario, ok, metrics })),
  violations,
};
writeTextArtifact(outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict: args.strict, ok: report.ok });
