#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SOURCE = 'shell/browser-v2/browser_shell_v2.ts';
const DEFAULT_COMPONENT = 'shell/browser-v2/BrowserShellV2.svelte';
const DEFAULT_RUNTIME = 'shell/browser-v2/src/browser_shell_v2_runtime.ts';
const DEFAULT_BUILD = 'shell/browser-v2/browser_shell_v2_build.ts';
const DEFAULT_SERVER = 'shell/browser-v2/browser_shell_v2_server.ts';
const DEFAULT_README = 'shell/browser-v2/README.md';
const DEFAULT_ARTIFACT = 'core/local/artifacts/browser_shell_v2_app/browser_shell_v2_app.js';
const DEFAULT_OUT_JSON = 'core/local/artifacts/browser_shell_v2_memory_surface_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/BROWSER_SHELL_V2_MEMORY_SURFACE_GUARD_CURRENT.md';

type Violation = {
  kind: string;
  path: string;
  detail: string;
};

const FORBIDDEN_TOKENS = [
  'localStorage',
  'sessionStorage',
  'indexedDB',
  'conversationCache',
  'window.__',
  'all_messages',
  'conversation_tree',
  'raw_tool_result',
  'raw_tool_input',
  'trace_body',
  'decision_trace',
  'plan_graph',
  'workflow_graph',
  'execution_observation',
  '_telemetrySnapshot',
  '_continuitySnapshot',
  'message_stream_buffer',
  'message_hydration_keys',
  'forcedHydrate',
  'client/runtime/systems/ui',
  'infring_static',
  'Alpine',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function read(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function readIfExists(relPath: string): string {
  try {
    return read(relPath);
  } catch {
    return '';
  }
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function validateFileTokens(violations: Violation[], pathRel: string, content: string): void {
  for (const token of FORBIDDEN_TOKENS) {
    if (content.includes(token)) {
      push(violations, 'forbidden_memory_surface_token', pathRel, `Forbidden token ${token}.`);
    }
  }
}

function validateBoundedArrays(violations: Violation[], sourcePath: string, runtimePath: string, componentPath: string, source: string, runtime: string, component: string): void {
  for (const [pathRel, content] of [[sourcePath, source], [runtimePath, runtime]] as Array<[string, string]>) {
    if (!content.includes('MESSAGE_WINDOW_LIMIT') || !content.includes('40')) {
      push(violations, 'missing_message_window_limit', pathRel, 'Browser V2 must cap default message windows at the socket projection boundary.');
    }
    if (!content.includes('slice(0, MESSAGE_WINDOW_LIMIT)')) {
      push(violations, 'missing_message_slice_guard', pathRel, 'Browser V2 must slice message rows before exposing them to UI state.');
    }
  }
  if (!runtime.includes('eventRows = [...eventRows, ...nextRows].slice(-20)')) {
    push(violations, 'missing_event_projection_limit', runtimePath, 'Runtime event projection must keep only a small tail window.');
  }
  if (!source.includes('this.eventRows = [...this.eventRows, ...nextRows].slice(-20)')) {
    push(violations, 'missing_smoke_event_projection_limit', sourcePath, 'Smoke/controller path must keep only a small event tail window.');
  }
  if (!runtime.includes('receiptRefs = Array.from(nextRefs).slice(-20)')) {
    push(violations, 'missing_receipt_projection_limit', runtimePath, 'Gateway receipt display must stay bounded.');
  }
  if (!source.includes('receipt_refs: this.receiptRefs.slice(0, 20)')) {
    push(violations, 'missing_snapshot_receipt_projection_limit', sourcePath, 'Snapshot receipt refs must stay bounded.');
  }
  if (!component.includes('{#each messages as message') || !component.includes('{#each receiptRefs as receiptRef')) {
    push(violations, 'missing_projection_only_render_loop', componentPath, 'Component must render bounded projected rows and receipt refs, not raw runtime objects.');
  }
}

function validateReadme(violations: Violation[], readmePath: string, readme: string): void {
  for (const phrase of ['bounded message window', 'raw tool payloads', 'full conversation trees', 'Gateway audit receipt ledger']) {
    if (!readme.includes(phrase)) {
      push(violations, 'memory_policy_not_documented', readmePath, `README must document ${phrase}.`);
    }
  }
}

function validate(includeControlledViolation: boolean, paths: Record<string, string>): Violation[] {
  const violations: Violation[] = [];
  const source = read(paths.source);
  const component = read(paths.component);
  const runtime = read(paths.runtime);
  const build = read(paths.build);
  const server = read(paths.server);
  const readme = read(paths.readme);
  const artifact = readIfExists(paths.artifact);
  const controlled = includeControlledViolation ? '\nlocalStorage\nall_messages\nraw_tool_result\n' : '';
  const executableFiles = [
    [paths.source, source + controlled],
    [paths.component, component],
    [paths.runtime, runtime],
    [paths.build, build],
    [paths.server, server],
  ] as Array<[string, string]>;
  if (artifact) executableFiles.push([paths.artifact, artifact]);
  for (const [pathRel, content] of executableFiles) validateFileTokens(violations, pathRel, content);
  validateBoundedArrays(violations, paths.source, paths.runtime, paths.component, source, runtime, component);
  validateReadme(violations, paths.readme, readme);
  return violations;
}

function markdown(report: any): string {
  const lines = [
    '# Browser Shell V2 Memory Surface Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    '',
    '## Violations',
  ];
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations as Violation[]) {
    lines.push(`- ${violation.kind}: ${violation.path} - ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const strict = parseBool(readFlag(argv, 'strict'), true);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const paths = {
  source: cleanText(readFlag(argv, 'source') || DEFAULT_SOURCE, 600),
  component: cleanText(readFlag(argv, 'component') || DEFAULT_COMPONENT, 600),
  runtime: cleanText(readFlag(argv, 'runtime') || DEFAULT_RUNTIME, 600),
  build: cleanText(readFlag(argv, 'build') || DEFAULT_BUILD, 600),
  server: cleanText(readFlag(argv, 'server') || DEFAULT_SERVER, 600),
  readme: cleanText(readFlag(argv, 'readme') || DEFAULT_README, 600),
  artifact: cleanText(readFlag(argv, 'artifact') || DEFAULT_ARTIFACT, 600),
};
const outJson = cleanText(readFlag(argv, 'out-json') || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const violations = validate(includeControlledViolation, paths);
const report = {
  ok: violations.length === 0,
  type: 'browser_shell_v2_memory_surface_guard',
  revision: currentRevision(ROOT),
  controlled_violation: includeControlledViolation,
  paths,
  violations,
};

writeTextArtifact(outMarkdown, markdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
