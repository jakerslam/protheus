#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const DEFAULT_SOURCE = 'shell/browser-v2/browser_shell_v2.ts';
const DEFAULT_COMPONENT = 'shell/browser-v2/BrowserShellV2.svelte';
const DEFAULT_STYLES = 'shell/browser-v2/browser_shell_v2.css';
const DEFAULT_RUNTIME = 'shell/browser-v2/src/browser_shell_v2_runtime.ts';
const DEFAULT_BUILD = 'shell/browser-v2/browser_shell_v2_build.ts';
const DEFAULT_SERVER = 'shell/browser-v2/browser_shell_v2_server.ts';
const DEFAULT_README = 'shell/browser-v2/README.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/browser_shell_v2_contract_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/BROWSER_SHELL_V2_CONTRACT_GUARD_CURRENT.md';

type Violation = { kind: string; path: string; detail: string };

const FORBIDDEN_TOKENS = [
  'http://127.0.0.1:4173',
  'client/runtime/systems/ui',
  'infring_static',
  'Alpine',
  'x-data',
  'localStorage',
  'sessionStorage',
  'conversationCache',
  'all_messages',
  'raw_tool_result',
  'decision_trace',
  'plan_graph',
  'workflow_graph',
];

function clean(value: unknown, max = 1000): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function readFlag(argv: string[], name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  for (let index = 0; index < argv.length; index += 1) {
    const token = clean(argv[index], 1200);
    if (token === `--${name}`) return clean(argv[index + 1], 1200);
    if (token.startsWith(prefix)) return clean(token.slice(prefix.length), 1200);
  }
  return fallback;
}

function parseBool(value: string, fallback = false): boolean {
  const normalized = clean(value, 32).toLowerCase();
  if (!normalized) return fallback;
  return ['1', 'true', 'yes', 'on'].includes(normalized);
}

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function read(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function writeJson(filePath: string, payload: unknown): void {
  const target = abs(filePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function writeMarkdown(filePath: string, sourcePath: string, violations: Violation[]): void {
  const lines = [
    '# Browser Shell V2 Contract Guard',
    '',
    `Source: \`${sourcePath}\``,
    `Pass: \`${violations.length === 0}\``,
    '',
    '## Violations',
  ];
  if (violations.length === 0) lines.push('- none');
  for (const violation of violations) lines.push(`- ${violation.kind}: ${violation.path} - ${violation.detail}`);
  const target = abs(filePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, `${lines.join('\n')}\n`, 'utf8');
}

function validatePackage(violations: Violation[]): void {
  const pkg = JSON.parse(read('package.json')) as { scripts?: Record<string, string> };
  for (const script of ['ops:browser-shell-v2:smoke', 'ops:browser-shell-v2:contract:guard', 'ops:browser-shell-v2:build', 'ops:browser-shell-v2:serve', 'ops:browser-shell-v2:serve-smoke']) {
    if (!pkg.scripts?.[script]) push(violations, 'missing_package_script', 'package.json', `Missing ${script}.`);
  }
}

function validate(sourcePath: string, componentPath: string, stylesPath: string, runtimePath: string, buildPath: string, serverPath: string, readmePath: string, includeControlledViolation: boolean): Violation[] {
  const violations: Violation[] = [];
  const source = `${read(sourcePath)}${includeControlledViolation ? '\nclient/runtime/systems/ui\n' : ''}`;
  const component = read(componentPath);
  const styles = read(stylesPath);
  const runtime = read(runtimePath);
  const build = read(buildPath);
  const server = read(serverPath);
  const readme = read(readmePath);
  const combined = `${source}\n${component}\n${styles}\n${runtime}\n${build}\n${server}`;
  if (!source.includes("from '../socket/client/shell_socket_gateway_client.ts'")) {
    push(violations, 'missing_socket_client_import', sourcePath, 'Browser Shell V2 must use the Shell Socket Gateway client.');
  }
  for (const method of ['getRuntimeStatus', 'listAgents', 'listSessions', 'getMessageWindow', 'getMessageDetail', 'subscribeEvents', 'search', 'submitIssue', 'submitApprovalDecision', 'setModel', 'setGitTree', 'submitInput']) {
    if (!source.includes(method)) push(violations, 'missing_socket_method', sourcePath, `Missing ${method}.`);
  }
  if (!source.includes('http://127.0.0.1:5173')) {
    push(violations, 'missing_gateway_default', sourcePath, 'Browser Shell V2 must default live mode to Gateway/backend 5173.');
  }
  if (!source.includes('MESSAGE_WINDOW_LIMIT') || !source.includes('40')) {
    push(violations, 'missing_bounded_window', sourcePath, 'Browser Shell V2 must keep message windows bounded.');
  }
  if (!component.includes('<script') || !component.includes('onSubmitInput') || !component.includes('{#each messages as message') || !component.includes('{#each agentRows as agent') || !component.includes('{#each sessionRows as session') || !component.includes('onOpenMessageDetail') || !component.includes('{#each eventRows as event') || !component.includes('{#each searchRows as result') || !component.includes('onSubmitIssue') || !component.includes('onSubmitApprovalDecision') || !component.includes('onSetModel') || !component.includes('onSetGitTree') || !component.includes('{#each modelRows as model') || !component.includes('{#each gitTreeRows as tree') || !component.includes('activeDetailPanel') || !component.includes('browser-shell-v2__detail-grid') || !component.includes('receiptRefs')) {
    push(violations, 'missing_svelte_projection_component', componentPath, 'Browser Shell V2 must expose a Svelte projection/input component with agent/session selection, lazy details, event projection, bounded search, issue submission, bounded model/git selectors, and selection request controls.');
  }
  if (!runtime.includes("from '../BrowserShellV2.svelte'") || !runtime.includes('ShellSocketGatewayClient') || !runtime.includes('submitInput') || !runtime.includes('selectAgent') || !runtime.includes('selectSession') || !runtime.includes('openMessageDetail') || !runtime.includes('refreshEvents') || !runtime.includes('client.search') || !runtime.includes('submitIssue') || !runtime.includes('client.submitApprovalDecision') || !runtime.includes('client.setModel') || !runtime.includes('client.setGitTree') || !runtime.includes('rowsFromSelectorOptions') || !runtime.includes('detailPanelFromProjection') || !runtime.includes('rowsFromDetailProjection') || !runtime.includes('slice(0, 12)') || !runtime.includes("['model_options', 'models', 'model_rows']") || !runtime.includes("['git_tree_options', 'git_trees', 'workspace_trees']") || !runtime.includes('EVENT_POLL_INTERVAL_MS') || !runtime.includes('eventRefreshInFlight') || !runtime.includes('startEventProjectionStream') || !runtime.includes('rememberReceiptRefs')) {
    push(violations, 'missing_browser_runtime_mount', runtimePath, 'Browser runtime must mount Svelte and use Shell Socket submitInput plus bounded selection/detail/event/search/issue/model/git-tree handlers.');
  }
  if (!build.includes('core/local/artifacts/browser_shell_v2_app') || !build.includes("from 'svelte/compiler'") || !build.includes('browser_shell_v2_app.js')) {
    push(violations, 'missing_browser_v2_build_target', buildPath, 'Build script must compile the Svelte source and write the browser artifact target.');
  }
  if (!build.includes('submitApprovalDecision') || !build.includes('/api/shell-socket/approvals/') || !build.includes('submit_approval_decision')) {
    push(violations, 'missing_browser_v2_approval_artifact_path', buildPath, 'Standalone Browser V2 artifact must keep approval decisions routed through the Shell Socket approval path.');
  }
  if (!build.includes('socketRequest') || !build.includes('/api/shell-socket/input') || !build.includes('/api/shell-socket/search')) {
    push(violations, 'missing_browser_v2_socket_artifact_paths', buildPath, 'Standalone Browser V2 artifact must submit input and bounded search through Shell Socket routes.');
  }
  if (!build.includes('rowsFromSelectorOptions') || !build.includes('data-model-id') || !build.includes('data-git-tree-id')) {
    push(violations, 'missing_browser_v2_selector_artifact_path', buildPath, 'Standalone Browser V2 artifact must render bounded model/git selector rows from projections.');
  }
  if (!build.includes('EVENT_POLL_INTERVAL_MS') || !build.includes('eventRefreshInFlight') || !build.includes('startEventProjectionStream')) {
    push(violations, 'missing_browser_v2_live_event_artifact_path', buildPath, 'Standalone Browser V2 artifact must keep live event updates bounded through Shell Socket polling.');
  }
  if (!build.includes('detailPanelFromProjection') || !build.includes('browser-shell-v2__detail-grid') || !build.includes('slice(0, 12)')) {
    push(violations, 'missing_browser_v2_detail_drawer_artifact_path', buildPath, 'Standalone Browser V2 artifact must render rich bounded detail drawers from detail projections.');
  }
  if (!server.includes('startBrowserShellV2Server') || !server.includes('browser_shell_v2_app') || !server.includes('serve-smoke') || !server.includes('127.0.0.1:5173') || !server.includes('waitForever') || !server.includes("process.on('SIGTERM'")) {
    push(violations, 'missing_browser_v2_static_server', serverPath, 'Server must serve the Browser Shell V2 artifact independently, stay resident in serve mode, shut down cleanly, and point clients at Gateway 5173.');
  }
  if (!styles.includes('.browser-shell-v2') || !styles.includes('var(--')) {
    push(violations, 'missing_clean_style_tokens', stylesPath, 'Browser Shell V2 styles must use clean component classes and reusable tokens.');
  }
  for (const token of FORBIDDEN_TOKENS) {
    if (combined.includes(token)) push(violations, 'browser_shell_v2_forbidden_dependency', sourcePath, `Forbidden token ${token}.`);
  }
  if (!readme.includes('Browser Shell V2') || !readme.includes('Shell Socket') || !readme.includes('Gateway') || !readme.includes('presentation/input plug') || !readme.includes('Local Serve')) {
    push(violations, 'browser_shell_v2_readme_incomplete', readmePath, 'README must explain Browser Shell V2, Shell Socket, Gateway, local serve, and presentation-only scope.');
  }
  validatePackage(violations);
  return violations;
}

const argv = process.argv.slice(2);
const sourcePath = readFlag(argv, 'source', DEFAULT_SOURCE);
const componentPath = readFlag(argv, 'component', DEFAULT_COMPONENT);
const stylesPath = readFlag(argv, 'styles', DEFAULT_STYLES);
const runtimePath = readFlag(argv, 'runtime', DEFAULT_RUNTIME);
const buildPath = readFlag(argv, 'build', DEFAULT_BUILD);
const serverPath = readFlag(argv, 'server', DEFAULT_SERVER);
const readmePath = readFlag(argv, 'readme', DEFAULT_README);
const outJson = readFlag(argv, 'out-json', DEFAULT_OUT_JSON);
const outMarkdown = readFlag(argv, 'out-markdown', DEFAULT_OUT_MARKDOWN);
const strict = parseBool(readFlag(argv, 'strict'), true);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const violations = validate(sourcePath, componentPath, stylesPath, runtimePath, buildPath, serverPath, readmePath, includeControlledViolation);
const result = {
  ok: violations.length === 0,
  type: 'browser_shell_v2_contract_guard',
  source_path: sourcePath,
  component_path: componentPath,
  styles_path: stylesPath,
  runtime_path: runtimePath,
  build_path: buildPath,
  server_path: serverPath,
  readme_path: readmePath,
  controlled_violation: includeControlledViolation,
  violations,
};
writeJson(outJson, result);
writeMarkdown(outMarkdown, sourcePath, violations);
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
if (strict && !result.ok) process.exitCode = 1;
