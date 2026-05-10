#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_INSTALLER = 'install.sh';
const DEFAULT_OUT_JSON = 'core/local/artifacts/gateway_shell_launch_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/GATEWAY_SHELL_LAUNCH_GUARD_CURRENT.md';

type Violation = { kind: string; path: string; detail: string };

const REQUIRED_TOKENS = [
  'INFRING_GATEWAY_DEFAULT_SHELL',
  'INFRING_GATEWAY_FALLBACK_SHELL',
  'INFRING_GATEWAY_LAUNCH_ON_START=1',
  '--shell=ui|ui-v2|terminal|legacy-ui|none',
  'infring_gateway_select_shell',
  'infring_gateway_configured_shell',
  'infring_gateway_launch_terminal_shell',
  'infring_gateway_prepare_browser_v2_shell',
  'browser_shell_v2_server.ts',
  'terminal_shell.ts',
  '--interactive=1',
  'INFRING_DASHBOARD_OPEN_ON_START=0',
  'INFRING_SHELL_SOCKET_URL:-http://127.0.0.1:5173',
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

function mutateControlled(source: string): string {
  return source
    .replace('infring_gateway_launch_terminal_shell', 'infring_gateway_launch_term_removed')
    .replace('infring_gateway_prepare_browser_v2_shell', 'infring_gateway_prepare_browser_v2_removed')
    .replace('INFRING_DASHBOARD_OPEN_ON_START=0', 'INFRING_DASHBOARD_OPEN_ON_START=1')
    .replace('--shell=ui|ui-v2|terminal|legacy-ui|none', '--shell=ui');
}

function validate(source: string, installerPath: string): Violation[] {
  const violations: Violation[] = [];
  for (const token of REQUIRED_TOKENS) {
    if (!source.includes(token)) push(violations, 'missing_gateway_shell_token', installerPath, `Missing ${token}.`);
  }
  if (!source.includes('write_shell_launch_config "$WORKSPACE_DIR"')) {
    push(violations, 'missing_setup_config_write', installerPath, 'Installer must write the setup shell default config.');
  }
  if (!source.includes('gateway_shell_override') || !source.includes('--terminal-shell|--terminal') || !source.includes('--ui-v2-shell|--ui-v2') || !source.includes('--no-shell')) {
    push(violations, 'missing_gateway_shell_override_parse', installerPath, 'Gateway wrapper must parse explicit shell override flags.');
  }
  if (!source.includes('infring_gateway_shell_uses_browser "$gateway_selected_shell"')) {
    push(violations, 'missing_browser_shell_gate', installerPath, 'Browser open must be gated by selected shell mode.');
  }
  if (!source.includes('echo "[infring gateway] shell: terminal"')) {
    push(violations, 'missing_terminal_shell_receipt', installerPath, 'Terminal launch must emit an operator-visible shell receipt.');
  }
  if (!source.includes('echo "[infring gateway] shell: ${shell_mode}"') || !source.includes('INFRING_BROWSER_SHELL_V2_PORT:-5273')) {
    push(violations, 'missing_browser_v2_shell_launch_receipt', installerPath, 'Browser Shell V2 launch must emit a shell receipt and use an isolated V2 port.');
  }
  return violations;
}

function markdown(installerPath: string, violations: Violation[]): string {
  const lines = [
    '# Gateway Shell Launch Guard',
    '',
    `Installer: \`${installerPath}\``,
    `Pass: \`${violations.length === 0}\``,
    '',
    '## Result',
  ];
  if (violations.length === 0) {
    lines.push('- `infring gateway` has setup-configured shell selection.');
    lines.push('- `--shell=terminal` disables browser auto-open and launches Terminal Shell through the Shell Socket.');
    lines.push('- `--shell=ui-v2` launches the clean Browser Shell V2 server without using the legacy dashboard host.');
  } else {
    for (const violation of violations) lines.push(`- ${violation.kind}: ${violation.path} - ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

function main(): void {
  const argv = process.argv.slice(2);
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  const installerPath = cleanText(readFlag(argv, 'installer') || DEFAULT_INSTALLER, 600);
  const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
  const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
  const controlled = parseBool(readFlag(argv, 'include-controlled-violation'), false);
  const source = controlled ? mutateControlled(read(installerPath)) : read(installerPath);
  const violations = validate(source, installerPath);
  const result = {
    ok: violations.length === 0,
    type: 'gateway_shell_launch_guard',
    revision: currentRevision(ROOT),
    installer_path: installerPath,
    controlled_violation: controlled,
    violations,
  };
  writeTextArtifact(outMarkdown, markdown(installerPath, violations));
  process.exitCode = emitStructuredResult(result, { outPath: outJson, strict: common.strict, ok: result.ok });
}

main();
