#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SERVICE = 'client/runtime/systems/ui/infring_static/js/shell/message_metadata_shell_services.ts';
const DEFAULT_SVELTE_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/message_meta_shell_svelte_source.ts';
const DEFAULT_SVELTE_BUNDLE = 'client/runtime/systems/ui/infring_static/js/svelte/message_meta_shell.bundle.ts';
const DEFAULT_CHAT = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts';
const DEFAULT_STATS_PART = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/160-runtime-events-and-render.part02.ts';
const DEFAULT_HOVER_PART = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/210-scroll-hover-sanitize.part01.ts';
const DEFAULT_META_PART = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/215-rendering-and-metadata-upgrades.ts';
const DEFAULT_REPORT_PART = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/216-message-report-issue-service.ts';
const DEFAULT_HTML_FILES = [
  'client/runtime/systems/ui/infring_static/index_body.html.parts/0003-body-part.html',
  'client/runtime/systems/ui/infring_static/index_body.html.parts/0005-body-part.html',
];
const DEFAULT_CSS = 'client/runtime/systems/ui/infring_static/css/components.css.parts/0006-components-part.css';
const DEFAULT_ROUTER = 'adapters/runtime/dashboard_asset_router.ts';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_message_metadata_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_MESSAGE_METADATA_OWNERSHIP_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  servicePath: string;
  svelteSourcePath: string;
  svelteBundlePath: string;
  chatPath: string;
  statsPartPath: string;
  hoverPartPath: string;
  metaPartPath: string;
  reportPartPath: string;
  cssPath: string;
  routerPath: string;
  htmlFiles: string[];
};

type Violation = {
  kind: string;
  path?: string;
  token?: string;
  detail: string;
};

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  const htmlOverride = readFlag(argv, 'html-files');
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    servicePath: cleanText(readFlag(argv, 'service') || DEFAULT_SERVICE, 400),
    svelteSourcePath: cleanText(readFlag(argv, 'svelte-source') || DEFAULT_SVELTE_SOURCE, 400),
    svelteBundlePath: cleanText(readFlag(argv, 'svelte-bundle') || DEFAULT_SVELTE_BUNDLE, 400),
    chatPath: cleanText(readFlag(argv, 'chat') || DEFAULT_CHAT, 400),
    statsPartPath: cleanText(readFlag(argv, 'stats-part') || DEFAULT_STATS_PART, 400),
    hoverPartPath: cleanText(readFlag(argv, 'hover-part') || DEFAULT_HOVER_PART, 400),
    metaPartPath: cleanText(readFlag(argv, 'meta-part') || DEFAULT_META_PART, 400),
    reportPartPath: cleanText(readFlag(argv, 'report-part') || DEFAULT_REPORT_PART, 400),
    cssPath: cleanText(readFlag(argv, 'css') || DEFAULT_CSS, 400),
    routerPath: cleanText(readFlag(argv, 'router') || DEFAULT_ROUTER, 400),
    htmlFiles: htmlOverride ? htmlOverride.split(',').map((value) => cleanText(value, 400)).filter(Boolean) : DEFAULT_HTML_FILES,
  };
}

function readText(path: string): string {
  return readFileSync(resolve(ROOT, path), 'utf8');
}

function requireExists(path: string, violations: Violation[]): boolean {
  if (existsSync(resolve(ROOT, path))) return true;
  violations.push({ kind: 'missing_metadata_ownership_source', path, detail: 'Required metadata ownership source is missing.' });
  return false;
}

function requireTokens(path: string, source: string, tokens: string[], kind: string, detail: string): Violation[] {
  return tokens
    .filter((token) => !source.includes(token))
    .map((token) => ({ kind, path, token, detail }));
}

function forbidTokens(path: string, source: string, tokens: string[], kind: string, detail: string): Violation[] {
  return tokens
    .filter((token) => source.includes(token))
    .map((token) => ({ kind, path, token, detail }));
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Message Metadata Ownership Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- checked_sources: ${payload.summary.checked_sources}`);
  lines.push(`- html_metadata_shells: ${payload.summary.html_metadata_shells}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.path || 'unknown'} ${violation.token || ''}`);
  }
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = readArgs(argv);
  const violations: Violation[] = [];
  const paths = [
    args.servicePath,
    args.svelteSourcePath,
    args.svelteBundlePath,
    args.chatPath,
    args.statsPartPath,
    args.hoverPartPath,
    args.metaPartPath,
    args.reportPartPath,
    args.cssPath,
    args.routerPath,
    ...args.htmlFiles,
  ];
  for (const path of paths) requireExists(path, violations);

  if (violations.length === 0) {
    const service = readText(args.servicePath);
    violations.push(...requireTokens(args.servicePath, service, [
      'services.messageMeta',
      'resolveIndex: resolveIndex',
      'retrySource: retrySource',
      'isLatestAgent: isLatestAgent',
      'canRetry: canRetry',
      'canReply: canReply',
      'canFork: canFork',
      'canReportIssue: canReportIssue',
      'responseTimeText: responseTimeText',
      'burnLabelText: burnLabelText',
      'viewModel: viewModel',
    ], 'missing_message_metadata_service_token', 'Message metadata truth and display eligibility must be centralized in shared shell services.'));

    const svelteSource = readText(args.svelteSourcePath);
    violations.push(...requireTokens(args.svelteSourcePath, svelteSource, [
      "customElement={{ tag: 'infring-message-meta-shell', shadow: 'none' }}",
      'createEventDispatcher',
      'export let state',
      "dispatch('message-meta-action'",
      'model.canReportIssue',
      'model.canRetry',
      'model.canReply',
      'model.canFork',
      'model.responseTime',
      'model.burnLabel',
    ], 'missing_svelte_message_metadata_token', 'Message metadata controls and indicators must be rendered by the Svelte metadata shell.'));
    violations.push(...forbidTokens(args.svelteSourcePath, svelteSource, ['<slot />'], 'slot_only_message_metadata_shell', 'The metadata shell must render controls itself, not pass Alpine markup through a slot.'));

    const svelteBundle = readText(args.svelteBundlePath);
    violations.push(...requireTokens(args.svelteBundlePath, svelteBundle, [
      'message-meta-action',
      'message-stat-burn',
      'infring-message-meta-shell',
    ], 'stale_message_metadata_bundle', 'The generated metadata shell bundle must contain the compiled metadata renderer.'));

    for (const htmlPath of args.htmlFiles) {
      const html = readText(htmlPath);
      violations.push(...requireTokens(htmlPath, html, [
        '<infring-message-meta-shell',
        'messageMetadataShellState(msg, idx,',
        'handleMessageMetaAction($event, msg, idx,',
      ], 'html_metadata_shell_not_wired', 'Chat templates must invoke the Svelte metadata shell with a service-backed state model.'));
      violations.push(...forbidTokens(htmlPath, html, [
        'messageCanReportIssueFromMeta',
        'messageCanRetryFromMeta',
        'messageCanReplyFromMeta',
        'messageCanForkFromMeta',
        'messageStatResponseTimeText',
        'messageStatBurnLabelText',
        'messageMetaVisible',
        'copyMessage(msg)',
        'toggleMessageTools(msg)',
      ], 'alpine_only_metadata_control_path', 'Chat metadata controls must not be rendered as direct Alpine-only button paths in HTML.'));
    }

    const chat = readText(args.chatPath);
    const metaPart = readText(args.metaPartPath);
    const statsPart = readText(args.statsPartPath);
    const hoverPart = readText(args.hoverPartPath);
    const reportPart = readText(args.reportPartPath);
    violations.push(...requireTokens(args.chatPath, chat, [
      'messageMetadataService',
      'messageMetadataShellState',
      'handleMessageMetaAction',
      'service.viewModel',
      'service.retrySource',
      'service.canRetry',
      'service.canReply',
      'service.canFork',
      'service.canReportIssue',
      'service.responseTimeText',
      'service.burnLabelText',
      'service.visible',
    ], 'chat_metadata_wrapper_not_delegated', 'Runtime chat compatibility methods must delegate metadata truth to shared shell services.'));
    violations.push(...requireTokens(args.metaPartPath, metaPart, [
      'messageMetadataService',
      'messageMetadataShellState',
      'handleMessageMetaAction',
      'service.viewModel',
      'service.retrySource',
      'service.canRetry',
      'service.canReply',
      'service.canFork',
    ], 'chat_part_metadata_wrapper_not_delegated', 'Segmented chat metadata part must mirror service-backed ownership.'));
    violations.push(...requireTokens(args.statsPartPath, statsPart, ['service.responseTimeText', 'service.burnLabelText'], 'stats_part_metadata_not_delegated', 'Metadata indicator text must delegate to shared shell services.'));
    violations.push(...requireTokens(args.hoverPartPath, hoverPart, ['service.visible'], 'hover_part_metadata_not_delegated', 'Metadata visibility must delegate to shared shell services.'));
    violations.push(...requireTokens(args.reportPartPath, reportPart, ['service.canReportIssue'], 'report_part_metadata_not_delegated', 'Report issue eligibility must delegate to shared shell services.'));
    violations.push(...requireTokens(args.cssPath, readText(args.cssPath), ['infring-message-meta-shell > .message-stats-row'], 'metadata_shell_css_missing', 'CSS must keep the Svelte-rendered metadata row on the same visual contract as the old row.'));
    violations.push(...requireTokens(args.routerPath, readText(args.routerPath), ['js/shell/message_metadata_shell_services'], 'metadata_service_not_loaded', 'The dashboard asset router must load the message metadata shell service before chat runtime code.'));
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_message_metadata_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      checked_sources: paths.length,
      html_metadata_shells: args.htmlFiles.length,
      violations: violations.length,
    },
    violations,
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  const exitCode = emitStructuredResult(payload, { outPath: args.outJson, strict: args.strict, ok: payload.ok });
  if (exitCode !== 0) process.exitCode = exitCode;
}

run().catch((error) => {
  console.error(error && error.stack ? error.stack : error);
  process.exitCode = 1;
});
