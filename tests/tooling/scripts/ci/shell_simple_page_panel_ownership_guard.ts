#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SERVICE = 'client/runtime/systems/ui/infring_static/js/shell/simple_page_panel_shell_services.ts';
const DEFAULT_ROUTER = 'adapters/runtime/dashboard_asset_router.ts';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_simple_page_panel_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_SIMPLE_PAGE_PANEL_OWNERSHIP_GUARD_CURRENT.md';

const PAGE_SHELLS = [
  { page: 'overview', tag: 'infring-overview-page-shell', source: 'overview_page_shell_svelte_source.ts', bundle: 'overview_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0002-body-part.html' },
  { page: 'agents', tag: 'infring-agents-page-shell', source: 'agents_page_shell_svelte_source.ts', bundle: 'agents_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0005-body-part.html' },
  { page: 'approvals', tag: 'infring-approvals-page-shell', source: 'approvals_page_shell_svelte_source.ts', bundle: 'approvals_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0008-body-part.html' },
  { page: 'workflows', tag: 'infring-workflows-page-shell', source: 'workflows_page_shell_svelte_source.ts', bundle: 'workflows_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0008-body-part.html' },
  { page: 'scheduler', tag: 'infring-scheduler-page-shell', source: 'scheduler_page_shell_svelte_source.ts', bundle: 'scheduler_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0009-body-part.html' },
  { page: 'channels', tag: 'infring-channels-page-shell', source: 'channels_page_shell_svelte_source.ts', bundle: 'channels_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0009-body-part.html' },
  { page: 'eyes', tag: 'infring-eyes-page-shell', source: 'eyes_page_shell_svelte_source.ts', bundle: 'eyes_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0010-body-part.html' },
  { page: 'skills', tag: 'infring-skills-page-shell', source: 'skills_page_shell_svelte_source.ts', bundle: 'skills_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0010-body-part.html' },
  { page: 'hands', tag: 'infring-hands-page-shell', source: 'hands_page_shell_svelte_source.ts', bundle: 'hands_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0011-body-part.html' },
  { page: 'settings', tag: 'infring-settings-page-shell', source: 'settings_page_shell_svelte_source.ts', bundle: 'settings_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0013-body-part.html' },
  { page: 'analytics', tag: 'infring-analytics-page-shell', source: 'analytics_page_shell_svelte_source.ts', bundle: 'analytics_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0014-body-part.html' },
  { page: 'sessions', tag: 'infring-sessions-page-shell', source: 'sessions_page_shell_svelte_source.ts', bundle: 'sessions_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0015-body-part.html' },
  { page: 'logs', tag: 'infring-logs-page-shell', source: 'logs_page_shell_svelte_source.ts', bundle: 'logs_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0015-body-part.html' },
  { page: 'comms', tag: 'infring-comms-page-shell', source: 'comms_page_shell_svelte_source.ts', bundle: 'comms_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0016-body-part.html' },
  { page: 'wizard', tag: 'infring-wizard-page-shell', source: 'wizard_page_shell_svelte_source.ts', bundle: 'wizard_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0016-body-part.html' },
  { page: 'runtime', tag: 'infring-runtime-page-shell', source: 'runtime_page_shell_svelte_source.ts', bundle: 'runtime_page_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0017-body-part.html' },
];

const TAB_SHELLS = [
  { page: 'workflows', tab: 'list', role: 'workflow-tab', tag: 'infring-workflows-list-tab-shell', source: 'workflows_list_tab_shell_svelte_source.ts', bundle: 'workflows_list_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0008-body-part.html' },
  { page: 'workflows', tab: 'builder', role: 'workflow-tab', tag: 'infring-workflows-builder-tab-shell', source: 'workflows_builder_tab_shell_svelte_source.ts', bundle: 'workflows_builder_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0008-body-part.html' },
  { page: 'scheduler', tab: 'jobs', role: 'scheduler-tab', tag: 'infring-scheduler-jobs-tab-shell', source: 'scheduler_jobs_tab_shell_svelte_source.ts', bundle: 'scheduler_jobs_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0009-body-part.html' },
  { page: 'scheduler', tab: 'triggers', role: 'scheduler-tab', tag: 'infring-scheduler-triggers-tab-shell', source: 'scheduler_triggers_tab_shell_svelte_source.ts', bundle: 'scheduler_triggers_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0009-body-part.html' },
  { page: 'scheduler', tab: 'history', role: 'scheduler-tab', tag: 'infring-scheduler-history-tab-shell', source: 'scheduler_history_tab_shell_svelte_source.ts', bundle: 'scheduler_history_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0009-body-part.html' },
  { page: 'skills', tab: 'installed', role: 'skills-tab', tag: 'infring-skills-installed-tab-shell', source: 'skills_installed_tab_shell_svelte_source.ts', bundle: 'skills_installed_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0010-body-part.html' },
  { page: 'skills', tab: 'clawhub', role: 'skills-tab', tag: 'infring-skills-clawhub-tab-shell', source: 'skills_clawhub_tab_shell_svelte_source.ts', bundle: 'skills_clawhub_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0010-body-part.html' },
  { page: 'skills', tab: 'mcp', role: 'skills-tab', tag: 'infring-skills-mcp-tab-shell', source: 'skills_mcp_tab_shell_svelte_source.ts', bundle: 'skills_mcp_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0011-body-part.html' },
  { page: 'skills', tab: 'create', role: 'skills-tab', tag: 'infring-skills-create-tab-shell', source: 'skills_create_tab_shell_svelte_source.ts', bundle: 'skills_create_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0011-body-part.html' },
  { page: 'hands', tab: 'available', role: 'hands-tab', tag: 'infring-hands-available-tab-shell', source: 'hands_available_tab_shell_svelte_source.ts', bundle: 'hands_available_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0011-body-part.html' },
  { page: 'hands', tab: 'active', role: 'hands-tab', tag: 'infring-hands-active-tab-shell', source: 'hands_active_tab_shell_svelte_source.ts', bundle: 'hands_active_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0011-body-part.html' },
  { page: 'settings', tab: 'providers', role: 'settings-tab', tag: 'infring-settings-providers-tab-shell', source: 'settings_providers_tab_shell_svelte_source.ts', bundle: 'settings_providers_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0013-body-part.html' },
  { page: 'settings', tab: 'models', role: 'settings-tab', tag: 'infring-settings-models-tab-shell', source: 'settings_models_tab_shell_svelte_source.ts', bundle: 'settings_models_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0013-body-part.html' },
  { page: 'settings', tab: 'tools', role: 'settings-tab', tag: 'infring-settings-tools-tab-shell', source: 'settings_tools_tab_shell_svelte_source.ts', bundle: 'settings_tools_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0013-body-part.html' },
  { page: 'settings', tab: 'info', role: 'settings-tab', tag: 'infring-settings-info-tab-shell', source: 'settings_info_tab_shell_svelte_source.ts', bundle: 'settings_info_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0013-body-part.html' },
  { page: 'settings', tab: 'config', role: 'settings-tab', tag: 'infring-settings-config-tab-shell', source: 'settings_config_tab_shell_svelte_source.ts', bundle: 'settings_config_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0013-body-part.html' },
  { page: 'settings', tab: 'security', role: 'settings-tab', tag: 'infring-settings-security-tab-shell', source: 'settings_security_tab_shell_svelte_source.ts', bundle: 'settings_security_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0013-body-part.html' },
  { page: 'settings', tab: 'network', role: 'settings-tab', tag: 'infring-settings-network-tab-shell', source: 'settings_network_tab_shell_svelte_source.ts', bundle: 'settings_network_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0014-body-part.html' },
  { page: 'settings', tab: 'budget', role: 'settings-tab', tag: 'infring-settings-budget-tab-shell', source: 'settings_budget_tab_shell_svelte_source.ts', bundle: 'settings_budget_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0014-body-part.html' },
  { page: 'settings', tab: 'migration', role: 'settings-tab', tag: 'infring-settings-migration-tab-shell', source: 'settings_migration_tab_shell_svelte_source.ts', bundle: 'settings_migration_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0014-body-part.html' },
  { page: 'analytics', tab: 'summary', role: 'analytics-tab', tag: 'infring-analytics-summary-tab-shell', source: 'analytics_summary_tab_shell_svelte_source.ts', bundle: 'analytics_summary_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0015-body-part.html' },
  { page: 'analytics', tab: 'by-model', role: 'analytics-tab', tag: 'infring-analytics-by-model-tab-shell', source: 'analytics_by_model_tab_shell_svelte_source.ts', bundle: 'analytics_by_model_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0015-body-part.html' },
  { page: 'analytics', tab: 'by-agent', role: 'analytics-tab', tag: 'infring-analytics-by-agent-tab-shell', source: 'analytics_by_agent_tab_shell_svelte_source.ts', bundle: 'analytics_by_agent_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0015-body-part.html' },
  { page: 'analytics', tab: 'costs', role: 'analytics-tab', tag: 'infring-analytics-costs-tab-shell', source: 'analytics_costs_tab_shell_svelte_source.ts', bundle: 'analytics_costs_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0015-body-part.html' },
  { page: 'sessions', tab: 'conversation', role: 'sessions-tab', tag: 'infring-sessions-conversation-tab-shell', source: 'sessions_conversation_tab_shell_svelte_source.ts', bundle: 'sessions_conversation_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0015-body-part.html' },
  { page: 'sessions', tab: 'memory', role: 'sessions-tab', tag: 'infring-sessions-memory-tab-shell', source: 'sessions_memory_tab_shell_svelte_source.ts', bundle: 'sessions_memory_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0015-body-part.html' },
  { page: 'logs', tab: 'live', role: 'logs-tab', tag: 'infring-logs-live-tab-shell', source: 'logs_live_tab_shell_svelte_source.ts', bundle: 'logs_live_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0016-body-part.html' },
  { page: 'logs', tab: 'audit', role: 'logs-tab', tag: 'infring-logs-audit-tab-shell', source: 'logs_audit_tab_shell_svelte_source.ts', bundle: 'logs_audit_tab_shell.bundle.ts', html: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0016-body-part.html' },
];

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  servicePath: string;
  routerPath: string;
};

type Violation = {
  kind: string;
  path?: string;
  token?: string;
  detail: string;
};

function args(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    servicePath: cleanText(readFlag(argv, 'service') || DEFAULT_SERVICE, 400),
    routerPath: cleanText(readFlag(argv, 'router') || DEFAULT_ROUTER, 400),
  };
}

function pathFor(source: string): string {
  return source.includes('/') ? source : `client/runtime/systems/ui/infring_static/js/svelte/${source}`;
}

function read(path: string): string {
  return readFileSync(resolve(ROOT, path), 'utf8');
}

function exists(path: string, violations: Violation[]) {
  if (existsSync(resolve(ROOT, path))) return true;
  violations.push({ kind: 'missing_simple_page_panel_source', path, detail: 'Required simple page panel source is missing.' });
  return false;
}

function requireTokens(path: string, source: string, tokens: string[], kind: string, detail: string): Violation[] {
  return tokens
    .filter((token) => !source.includes(token))
    .map((token) => ({ kind, path, token, detail }));
}

function htmlOpenToken(row: any): string {
  var attrs = `shellprimitive="simple-page-panel" pageid="${row.page}"`;
  if (row.tab) attrs += ` tabid="${row.tab}"`;
  attrs += ` panelrole="${row.role || 'page'}" routecontract="${row.page}${row.tab ? ':' + row.tab : ''}" parentowneddata="true"`;
  return `<${row.tag} ${attrs}`;
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Simple Page Panel Ownership Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- page_shells: ${payload.summary.page_shells}`);
  lines.push(`- tab_shells: ${payload.summary.tab_shells}`);
  lines.push(`- checked_sources: ${payload.summary.checked_sources}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) lines.push(`- ${violation.kind}: ${violation.path || 'unknown'} ${violation.token || ''}`);
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const parsed = args(argv);
  const violations: Violation[] = [];
  const rows = [...PAGE_SHELLS, ...TAB_SHELLS];
  const paths = [parsed.servicePath, parsed.routerPath, ...rows.flatMap((row) => [pathFor(row.source), pathFor(row.bundle), row.html])];
  for (const path of Array.from(new Set(paths))) exists(path, violations);

  if (violations.length === 0) {
    const service = read(parsed.servicePath);
    violations.push(...requireTokens(parsed.servicePath, service, [
      'services.simplePagePanel',
      'pageIds: pageIds',
      'tabIds: tabIds',
      'pageSpec: pageSpec',
      'tabSpec: tabSpec',
      'routeContract: routeContract',
      'shellTagFor: shellTagFor',
      'isKnownPanel: isKnownPanel',
      'infring-overview-page-shell',
      'infring-runtime-page-shell',
      'infring-logs-audit-tab-shell',
      'infring-settings-migration-tab-shell',
    ], 'missing_simple_page_panel_service_token', 'Simple page panel route/tab metadata must be centralized in the shared Shell simplePagePanel service.'));

    violations.push(...requireTokens(parsed.routerPath, read(parsed.routerPath), [
      "readForkScript(staticDir, 'js/shell/simple_page_panel_shell_services')",
      "readForkScript(staticDir, 'js/svelte/overview_page_shell.bundle')",
      "readForkScript(staticDir, 'js/svelte/runtime_page_shell.bundle')",
      "readForkScript(staticDir, 'js/svelte/logs_audit_tab_shell.bundle')",
      "readForkScript(staticDir, 'js/svelte/settings_migration_tab_shell.bundle')",
    ], 'simple_page_panel_router_not_loaded', 'The dashboard asset router must load simple page panel service and same-dashboard Svelte page/tab bundles.'));

    for (const row of PAGE_SHELLS) {
      const sourcePath = pathFor(row.source);
      const bundlePath = pathFor(row.bundle);
      const source = read(sourcePath);
      const bundle = read(bundlePath);
      violations.push(...requireTokens(sourcePath, source, [
        "export let shellPrimitive = 'simple-page-panel'",
        `export let pageId = '${row.page}'`,
        "export let panelRole = 'page'",
        `export let routeContract = '${row.page}'`,
        'export let parentOwnedData = true',
        '<slot />',
      ], 'page_shell_not_simple_page_panel_owned', 'Page Svelte shell must advertise simple-page-panel ownership while preserving slotted existing markup.'));
      violations.push(...requireTokens(bundlePath, bundle, ['shellPrimitive', 'pageId', 'panelRole', 'routeContract', 'parentOwnedData'], 'stale_simple_page_panel_bundle', 'Generated page shell bundle is stale or missing ownership props.'));
      violations.push(...requireTokens(row.html, read(row.html), [htmlOpenToken(row)], 'page_shell_host_missing_contract', 'Live same-dashboard page host must pass the simple-page-panel route contract props.'));
    }

    for (const row of TAB_SHELLS) {
      const sourcePath = pathFor(row.source);
      const bundlePath = pathFor(row.bundle);
      const source = read(sourcePath);
      const bundle = read(bundlePath);
      violations.push(...requireTokens(sourcePath, source, [
        "export let shellPrimitive = 'simple-page-panel'",
        `export let pageId = '${row.page}'`,
        `export let tabId = '${row.tab}'`,
        `export let panelRole = '${row.role}'`,
        `export let routeContract = '${row.page}:${row.tab}'`,
        'export let parentOwnedData = true',
        '<slot />',
      ], 'tab_shell_not_simple_page_panel_owned', 'Settings/workflow tab Svelte shell must advertise simple-page-panel ownership while preserving slotted existing markup.'));
      violations.push(...requireTokens(bundlePath, bundle, ['shellPrimitive', 'pageId', 'tabId', 'panelRole', 'routeContract', 'parentOwnedData'], 'stale_simple_page_panel_tab_bundle', 'Generated tab shell bundle is stale or missing ownership props.'));
      violations.push(...requireTokens(row.html, read(row.html), [htmlOpenToken(row)], 'tab_shell_host_missing_contract', 'Live same-dashboard tab host must pass the simple-page-panel route/tab contract props.'));
    }
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_simple_page_panel_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      page_shells: PAGE_SHELLS.length,
      tab_shells: TAB_SHELLS.length,
      checked_sources: Array.from(new Set([parsed.servicePath, parsed.routerPath, ...rows.flatMap((row) => [pathFor(row.source), pathFor(row.bundle), row.html])])).length,
      violations: violations.length,
    },
    violations,
  };
  writeTextArtifact(parsed.outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: parsed.outJson });
  if (!payload.ok && parsed.strict) process.exitCode = 1;
}

run().catch((error) => {
  const payload = { ok: false, type: 'shell_simple_page_panel_ownership_guard', error: error instanceof Error ? error.message : String(error) };
  emitStructuredResult(payload, { ok: false, outPath: DEFAULT_OUT_JSON });
  process.exitCode = 1;
});
