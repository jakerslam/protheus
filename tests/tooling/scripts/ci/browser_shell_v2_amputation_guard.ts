#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_BROWSER_DIR = 'shell/browser-v2';
const DEFAULT_SOCKET_DIR = 'shell/socket';
const DEFAULT_OUT_JSON = 'core/local/artifacts/browser_shell_v2_amputation_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/BROWSER_SHELL_V2_AMPUTATION_GUARD_CURRENT.md';

type Violation = {
  kind: string;
  path: string;
  detail: string;
};

const REQUIRED_FILES = [
  'shell/browser-v2/BrowserShellV2.svelte',
  'shell/browser-v2/browser_shell_v2.ts',
  'shell/browser-v2/src/browser_shell_v2_runtime.ts',
  'shell/browser-v2/browser_shell_v2_build.ts',
  'shell/browser-v2/browser_shell_v2_server.ts',
  'shell/browser-v2/browser_shell_v2.css',
  'shell/socket/client/shell_socket_gateway_client.ts',
  'shell/socket/contract/shell_socket_contract.json',
];

const FORBIDDEN_DEPENDENCIES = [
  'client/runtime/systems/ui',
  'infring_static',
  'dashboard_sveltekit',
  'x-data',
  'Alpine.store',
  'window.app',
  'window.__infring',
  'conversationCache',
  'http://127.0.0.1:4173',
  '/dashboard#chat',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function walkFiles(dirRel: string): string[] {
  const dirAbs = abs(dirRel);
  if (!fs.existsSync(dirAbs)) return [];
  const out: string[] = [];
  for (const entry of fs.readdirSync(dirAbs, { withFileTypes: true })) {
    const rel = path.join(dirRel, entry.name);
    if (entry.isDirectory()) {
      out.push(...walkFiles(rel));
    } else if (entry.isFile()) {
      out.push(rel);
    }
  }
  return out.sort();
}

function read(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function validateRequiredFiles(violations: Violation[]): void {
  for (const file of REQUIRED_FILES) {
    if (!fs.existsSync(abs(file))) push(violations, 'missing_required_shell2_file', file, 'Browser V2 must remain independently buildable through Shell Socket files.');
  }
}

function validateDependencyTokens(violations: Violation[], files: string[], includeControlledViolation: boolean): void {
  for (const file of files) {
    const content = `${read(file)}${includeControlledViolation && file.endsWith('browser_shell_v2.ts') ? '\nclient/runtime/systems/ui\nhttp://127.0.0.1:4173\n' : ''}`;
    for (const token of FORBIDDEN_DEPENDENCIES) {
      if (content.includes(token)) push(violations, 'legacy_dashboard_dependency', file, `Forbidden dependency token ${token}.`);
    }
  }
}

function validateStandaloneServer(violations: Violation[]): void {
  const serverPath = 'shell/browser-v2/browser_shell_v2_server.ts';
  const buildPath = 'shell/browser-v2/browser_shell_v2_build.ts';
  if (!fs.existsSync(abs(serverPath)) || !fs.existsSync(abs(buildPath))) return;
  const server = read(serverPath);
  const build = read(buildPath);
  if (!server.includes('browser_shell_v2_app') || !server.includes('127.0.0.1:5173')) {
    push(violations, 'missing_standalone_server_contract', serverPath, 'Browser V2 server must serve its own artifact and point at Gateway 5173.');
  }
  if (!build.includes('core/local/artifacts/browser_shell_v2_app') || !build.includes('BrowserShellV2.svelte')) {
    push(violations, 'missing_standalone_build_contract', buildPath, 'Browser V2 build must compile the clean Svelte plug into its own artifact.');
  }
}

function markdown(report: any): string {
  const lines = [
    '# Browser Shell V2 Amputation Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    '',
    '## Violations',
  ];
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations as Violation[]) lines.push(`- ${violation.kind}: ${violation.path} - ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const browserDir = cleanText(readFlag(argv, 'browser-dir') || DEFAULT_BROWSER_DIR, 600);
const socketDir = cleanText(readFlag(argv, 'socket-dir') || DEFAULT_SOCKET_DIR, 600);
const strict = parseBool(readFlag(argv, 'strict'), true);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const outJson = cleanText(readFlag(argv, 'out-json') || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const files = [...walkFiles(browserDir), ...walkFiles(socketDir)].filter((file) => /\.(ts|svelte|css|json|html)$/.test(file));
const violations: Violation[] = [];

validateRequiredFiles(violations);
validateDependencyTokens(violations, files, includeControlledViolation);
validateStandaloneServer(violations);

const report = {
  ok: violations.length === 0,
  type: 'browser_shell_v2_amputation_guard',
  revision: currentRevision(ROOT),
  controlled_violation: includeControlledViolation,
  browser_dir: browserDir,
  socket_dir: socketDir,
  scanned_file_count: files.length,
  violations,
};

writeTextArtifact(outMarkdown, markdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
