#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SOURCE = 'shell/terminal/terminal_shell.ts';
const DEFAULT_RENDERER = 'shell/terminal/terminal_output_renderer.ts';
const DEFAULT_NODE_FETCH = 'shell/terminal/terminal_node_fetch.ts';
const DEFAULT_REPLY_PROJECTION = 'shell/terminal/terminal_reply_projection.ts';
const DEFAULT_README = 'shell/terminal/README.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/terminal_shell_contract_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/TERMINAL_SHELL_CONTRACT_GUARD_CURRENT.md';

type Violation = { kind: string; path: string; detail: string };

const FORBIDDEN_TOKENS = [
  'http://127.0.0.1:4173',
  'client/runtime/systems/ui',
  'infring_static',
  'Alpine',
  'Svelte',
  'localStorage',
  'sessionStorage',
  'document.',
  'window.',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function read(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function validatePackage(violations: Violation[]): void {
  const pkg = JSON.parse(read('package.json')) as { scripts?: Record<string, string> };
  for (const script of [
    'ops:terminal-shell:contract:guard',
    'ops:terminal-shell:interactive-smoke',
    'ops:terminal-shell:response-test',
    'ops:terminal-shell:live-response-test',
    'ops:terminal-shell:render-fixture',
  ]) {
    if (!pkg.scripts?.[script]) push(violations, 'missing_package_script', 'package.json', `Missing ${script}.`);
  }
}

function validateSource(sourcePath: string, rendererPath: string, nodeFetchPath: string, replyProjectionPath: string, readmePath: string, includeControlledViolation: boolean): Violation[] {
  const violations: Violation[] = [];
  const source = `${read(sourcePath)}${includeControlledViolation ? '\nhttp://127.0.0.1:4173\n' : ''}`;
  const renderer = read(rendererPath);
  const nodeFetch = read(nodeFetchPath);
  const replyProjection = read(replyProjectionPath);
  const readme = read(readmePath);
  if (!source.includes("from '../socket/client/shell_socket_gateway_client.ts'")) {
    push(violations, 'missing_socket_client_import', sourcePath, 'Terminal Shell must use the Shell Socket Gateway client.');
  }
  if (!source.includes('getRuntimeStatus')) {
    push(violations, 'missing_response_capability', sourcePath, 'Terminal Shell response test must call get_runtime_status.');
  }
  if (!source.includes('startInteractive') || !`${source}\n${replyProjection}`.includes('submitInput') || !source.includes('SIGTSTP')) {
    push(violations, 'missing_interactive_terminal_loop', sourcePath, 'Terminal Shell must expose an interactive loop, submit input through Shell Socket, and stop on Ctrl-Z/SIGTSTP.');
  }
  if (!source.includes('http://127.0.0.1:5173')) {
    push(violations, 'missing_gateway_default', sourcePath, 'Terminal Shell must default live mode to Gateway/backend 5173.');
  }
  if (!renderer.includes('renderTerminalBlocks') || !renderer.includes('terminalRenderFixtureBlocks')) {
    push(violations, 'missing_terminal_renderer', rendererPath, 'Terminal Shell renderer must expose block rendering and a fixture.');
  }
  if (!renderer.includes('Infring')) {
    push(violations, 'missing_infring_brand', rendererPath, 'Terminal Shell renderer must render Infring-branded output.');
  }
  if (renderer.includes('ShellSocketGatewayClient') || renderer.includes('fetch(')) {
    push(violations, 'renderer_has_transport_authority', rendererPath, 'Terminal renderer must stay presentation-only and cannot own transport.');
  }
  for (const token of FORBIDDEN_TOKENS) {
    if (`${source}\n${renderer}\n${nodeFetch}\n${replyProjection}`.includes(token)) push(violations, 'terminal_shell_forbidden_dependency', sourcePath, `Forbidden token ${token}.`);
  }
  if (!readme.includes('Terminal Shell') || !readme.includes('Shell Socket') || !readme.includes('Gateway') || !readme.includes('presentation-only')) {
    push(violations, 'terminal_shell_readme_incomplete', readmePath, 'README must explain Terminal Shell, Shell Socket, and Gateway boundaries.');
  }
  validatePackage(violations);
  return violations;
}

function markdown(sourcePath: string, violations: Violation[]): string {
  const lines = [
    '# Terminal Shell Contract Guard',
    '',
    `Source: \`${sourcePath}\``,
    `Pass: \`${violations.length === 0}\``,
    '',
    '## Violations',
  ];
  if (violations.length === 0) lines.push('- none');
  for (const violation of violations) lines.push(`- ${violation.kind}: ${violation.path} - ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
const sourcePath = cleanText(readFlag(argv, 'source') || DEFAULT_SOURCE, 600);
const rendererPath = cleanText(readFlag(argv, 'renderer') || DEFAULT_RENDERER, 600);
const nodeFetchPath = cleanText(readFlag(argv, 'node-fetch') || DEFAULT_NODE_FETCH, 600);
const replyProjectionPath = cleanText(readFlag(argv, 'reply-projection') || DEFAULT_REPLY_PROJECTION, 600);
const readmePath = cleanText(readFlag(argv, 'readme') || DEFAULT_README, 600);
const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const violations = validateSource(sourcePath, rendererPath, nodeFetchPath, replyProjectionPath, readmePath, includeControlledViolation);
const result = {
  ok: violations.length === 0,
  type: 'terminal_shell_contract_guard',
  revision: currentRevision(ROOT),
  source_path: sourcePath,
  renderer_path: rendererPath,
  readme_path: readmePath,
  controlled_violation: includeControlledViolation,
  violations,
};
writeTextArtifact(outMarkdown, markdown(sourcePath, violations));
process.exitCode = emitStructuredResult(result, { outPath: outJson, strict: common.strict, ok: result.ok });
