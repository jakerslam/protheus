#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SOCKET_CONTRACT = 'shell/socket/contract/shell_socket_contract.json';
const DEFAULT_SOCKET_CONTRACT_GUARD = 'core/local/artifacts/shell_socket_contract_guard_current.json';
const DEFAULT_GATEWAY_ROUTE_GUARD = 'core/local/artifacts/shell_socket_gateway_route_guard_current.json';
const DEFAULT_LIVE_PROBE = 'core/local/artifacts/shell_socket_live_probe_current.json';
const DEFAULT_HEADLESS_PROBE = 'core/local/artifacts/shell_socket_headless_probe_current.json';
const DEFAULT_BROWSER_GUARD = 'core/local/artifacts/browser_shell_v2_contract_guard_current.json';
const DEFAULT_BROWSER_SMOKE = 'core/local/artifacts/browser_shell_v2_smoke_current.json';
const DEFAULT_BROWSER_BUILD = 'core/local/artifacts/browser_shell_v2_build_current.json';
const DEFAULT_BROWSER_SERVE_SMOKE = 'core/local/artifacts/browser_shell_v2_serve_smoke_current.json';
const DEFAULT_BROWSER_MEMORY_GUARD = 'core/local/artifacts/browser_shell_v2_memory_surface_guard_current.json';
const DEFAULT_BROWSER_AMPUTATION_GUARD = 'core/local/artifacts/browser_shell_v2_amputation_guard_current.json';
const DEFAULT_BROWSER_ACCESSIBILITY_GUARD = 'core/local/artifacts/browser_shell_v2_accessibility_guard_current.json';
const DEFAULT_BROWSER_VISUAL_PARITY_GUARD = 'core/local/artifacts/browser_shell_v2_visual_parity_guard_current.json';
const DEFAULT_TERMINAL_GUARD = 'core/local/artifacts/terminal_shell_contract_guard_current.json';
const DEFAULT_TERMINAL_RESPONSE_TEST = 'core/local/artifacts/terminal_shell_response_test_current.json';
const DEFAULT_TERMINAL_INTERACTIVE_SMOKE = 'core/local/artifacts/terminal_shell_interactive_smoke_current.json';
const DEFAULT_LEGACY_MANIFEST = 'shell/legacy/legacy_browser_shell_manifest.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell2_graduation_status_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL2_GRADUATION_STATUS_CURRENT.md';

type Check = {
  id: string;
  ok: boolean;
  state: 'pass' | 'fail' | 'missing';
  required: boolean;
  detail: string;
  artifact?: string;
};

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readJson(relPath: string): any {
  try {
    return JSON.parse(fs.readFileSync(abs(relPath), 'utf8'));
  } catch {
    return null;
  }
}

function exists(relPath: string): boolean {
  return fs.existsSync(abs(relPath));
}

function checkArtifact(id: string, artifactPath: string, label: string): Check {
  if (!exists(artifactPath)) {
    return {
      id,
      ok: false,
      state: 'missing',
      required: true,
      detail: `${label} artifact is missing: ${artifactPath}`,
      artifact: artifactPath,
    };
  }
  const payload = readJson(artifactPath);
  const ok = payload?.ok === true;
  return {
    id,
    ok,
    state: ok ? 'pass' : 'fail',
    required: true,
    detail: ok ? `${label} passed.` : `${label} did not pass.`,
    artifact: artifactPath,
  };
}

function advisory(id: string, ok: boolean, detail: string): Check {
  return { id, ok, state: ok ? 'pass' : 'fail', required: false, detail };
}

function liveParityCheck(liveProbePath: string): Check {
  if (!exists(liveProbePath)) {
    return {
      id: 'live_socket_parity',
      ok: false,
      state: 'missing',
      required: true,
      detail: `Live probe artifact is missing: ${liveProbePath}`,
      artifact: liveProbePath,
    };
  }
  const payload = readJson(liveProbePath);
  const complete = payload?.live_parity_complete === true;
  const failed = Array.isArray(payload?.failed_live_capabilities) ? payload.failed_live_capabilities : [];
  const missing = Array.isArray(payload?.missing_live_capabilities) ? payload.missing_live_capabilities : [];
  const liveAvailable = payload?.live_available === true;
  const detail = complete
    ? 'Live Shell Socket parity is complete.'
    : `Live Shell Socket parity incomplete; live_available=${liveAvailable}; failed=${failed.join(',') || 'none'}; missing=${missing.join(',') || 'none'}.`;
  return { id: 'live_socket_parity', ok: complete, state: complete ? 'pass' : 'fail', required: true, detail, artifact: liveProbePath };
}

function legacyQuarantineCheck(manifestPath: string): Check {
  if (!exists(manifestPath)) {
    return {
      id: 'legacy_quarantine',
      ok: false,
      state: 'missing',
      required: true,
      detail: `Legacy manifest is missing: ${manifestPath}`,
      artifact: manifestPath,
    };
  }
  const payload = readJson(manifestPath);
  const ok =
    payload?.status === 'legacy' &&
    payload?.canonical === false &&
    payload?.may_claim_socket_parity === false &&
    payload?.may_bypass_gateway === false &&
    payload?.may_receive_new_features === false;
  return {
    id: 'legacy_quarantine',
    ok,
    state: ok ? 'pass' : 'fail',
    required: true,
    detail: ok ? 'Legacy Browser Shell is quarantined and cannot claim socket parity.' : 'Legacy Browser Shell quarantine manifest is weak.',
    artifact: manifestPath,
  };
}

function socketContractCheck(contractPath: string): Check {
  if (!exists(contractPath)) {
    return {
      id: 'socket_contract',
      ok: false,
      state: 'missing',
      required: true,
      detail: `Socket contract is missing: ${contractPath}`,
      artifact: contractPath,
    };
  }
  const payload = readJson(contractPath);
  const capabilityCount = Array.isArray(payload?.capabilities) ? payload.capabilities.length : 0;
  const ok = payload?.type === 'shell_socket_contract' && capabilityCount >= 13;
  return {
    id: 'socket_contract',
    ok,
    state: ok ? 'pass' : 'fail',
    required: true,
    detail: ok ? `Socket contract declares ${capabilityCount} capabilities.` : 'Socket contract is incomplete.',
    artifact: contractPath,
  };
}

function nextActions(checks: Check[]): string[] {
  const failedIds = new Set(checks.filter((check) => check.required && !check.ok).map((check) => check.id));
  const actions: string[] = [];
  if (failedIds.has('live_socket_parity')) {
    actions.push('Restart the Gateway only when safe, then rerun the live Shell Socket probe to prove the current Rust route set.');
  }
  if (
    failedIds.has('browser_v2_contract') ||
    failedIds.has('browser_v2_smoke') ||
    failedIds.has('browser_v2_build') ||
    failedIds.has('browser_v2_serve_smoke') ||
    failedIds.has('browser_v2_memory_surface') ||
    failedIds.has('browser_v2_amputation') ||
    failedIds.has('browser_v2_accessibility') ||
    failedIds.has('browser_v2_visual_parity')
  ) {
    actions.push('Stabilize Browser Shell V2 against the socket contract before adding more visual parity.');
  }
  if (failedIds.has('terminal_contract') || failedIds.has('terminal_response_test') || failedIds.has('terminal_interactive_smoke')) {
    actions.push('Stabilize Terminal Shell as the first alternate plug proving the socket can serve non-browser shells.');
  }
  if (failedIds.has('legacy_quarantine')) {
    actions.push('Repair the legacy manifest so Shell 1.0 cannot claim canonical or socket parity status.');
  }
  actions.push('Keep legacy dashboard work to critical fixes only; new shell capability should land behind Shell Socket plugs.');
  actions.push('Before retiring legacy, keep parity work evidence-based and avoid reimporting legacy state.');
  return actions;
}

function markdown(report: any): string {
  const lines = [
    '# Shell 2.0 Graduation Status',
    '',
    `ok: ${report.ok}`,
    `ready_to_retire_legacy: ${report.ready_to_retire_legacy}`,
    `revision: ${report.revision}`,
    '',
    '## Checks',
  ];
  for (const check of report.checks as Check[]) {
    const required = check.required ? 'required' : 'advisory';
    lines.push(`- ${check.state}: ${check.id} (${required}) - ${check.detail}`);
  }
  lines.push('', '## Blocking Items');
  if (report.blocking_items.length === 0) lines.push('- none');
  for (const item of report.blocking_items as string[]) lines.push(`- ${item}`);
  lines.push('', '## Next Actions');
  for (const item of report.next_actions as string[]) lines.push(`- ${item}`);
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
const strict = parseBool(readFlag(argv, 'strict'), common.strict);
const socketContractPath = cleanText(readFlag(argv, 'socket-contract') || DEFAULT_SOCKET_CONTRACT, 600);
const socketContractGuardPath = cleanText(readFlag(argv, 'socket-contract-guard') || DEFAULT_SOCKET_CONTRACT_GUARD, 600);
const gatewayRouteGuardPath = cleanText(readFlag(argv, 'gateway-route-guard') || DEFAULT_GATEWAY_ROUTE_GUARD, 600);
const liveProbePath = cleanText(readFlag(argv, 'live-probe') || DEFAULT_LIVE_PROBE, 600);
const headlessProbePath = cleanText(readFlag(argv, 'headless-probe') || DEFAULT_HEADLESS_PROBE, 600);
const browserGuardPath = cleanText(readFlag(argv, 'browser-guard') || DEFAULT_BROWSER_GUARD, 600);
const browserSmokePath = cleanText(readFlag(argv, 'browser-smoke') || DEFAULT_BROWSER_SMOKE, 600);
const browserBuildPath = cleanText(readFlag(argv, 'browser-build') || DEFAULT_BROWSER_BUILD, 600);
const browserServeSmokePath = cleanText(readFlag(argv, 'browser-serve-smoke') || DEFAULT_BROWSER_SERVE_SMOKE, 600);
const browserMemoryGuardPath = cleanText(readFlag(argv, 'browser-memory-guard') || DEFAULT_BROWSER_MEMORY_GUARD, 600);
const browserAmputationGuardPath = cleanText(readFlag(argv, 'browser-amputation-guard') || DEFAULT_BROWSER_AMPUTATION_GUARD, 600);
const browserAccessibilityGuardPath = cleanText(readFlag(argv, 'browser-accessibility-guard') || DEFAULT_BROWSER_ACCESSIBILITY_GUARD, 600);
const browserVisualParityGuardPath = cleanText(readFlag(argv, 'browser-visual-parity-guard') || DEFAULT_BROWSER_VISUAL_PARITY_GUARD, 600);
const terminalGuardPath = cleanText(readFlag(argv, 'terminal-guard') || DEFAULT_TERMINAL_GUARD, 600);
const terminalResponsePath = cleanText(readFlag(argv, 'terminal-response') || DEFAULT_TERMINAL_RESPONSE_TEST, 600);
const terminalInteractivePath = cleanText(readFlag(argv, 'terminal-interactive') || DEFAULT_TERMINAL_INTERACTIVE_SMOKE, 600);
const legacyManifestPath = cleanText(readFlag(argv, 'legacy-manifest') || DEFAULT_LEGACY_MANIFEST, 600);
const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);

const checks = [
  socketContractCheck(socketContractPath),
  checkArtifact('socket_contract_guard', socketContractGuardPath, 'Shell Socket contract guard'),
  checkArtifact('gateway_route_guard', gatewayRouteGuardPath, 'Shell Socket Gateway route guard'),
  checkArtifact('headless_socket_parity', headlessProbePath, 'Headless Shell Socket parity'),
  liveParityCheck(liveProbePath),
  checkArtifact('browser_v2_contract', browserGuardPath, 'Browser Shell V2 contract'),
  checkArtifact('browser_v2_smoke', browserSmokePath, 'Browser Shell V2 smoke'),
  checkArtifact('browser_v2_build', browserBuildPath, 'Browser Shell V2 build'),
  checkArtifact('browser_v2_serve_smoke', browserServeSmokePath, 'Browser Shell V2 serve smoke'),
  checkArtifact('browser_v2_memory_surface', browserMemoryGuardPath, 'Browser Shell V2 memory surface guard'),
  checkArtifact('browser_v2_amputation', browserAmputationGuardPath, 'Browser Shell V2 amputation guard'),
  checkArtifact('browser_v2_accessibility', browserAccessibilityGuardPath, 'Browser Shell V2 accessibility guard'),
  checkArtifact('browser_v2_visual_parity', browserVisualParityGuardPath, 'Browser Shell V2 visual parity guard'),
  checkArtifact('terminal_contract', terminalGuardPath, 'Terminal Shell contract'),
  checkArtifact('terminal_response_test', terminalResponsePath, 'Terminal Shell response test'),
  checkArtifact('terminal_interactive_smoke', terminalInteractivePath, 'Terminal Shell interactive smoke'),
  legacyQuarantineCheck(legacyManifestPath),
  advisory('browser_visual_parity', true, 'Browser Shell V2 has baseline visual parity coverage; full product parity remains a feature-by-feature migration concern.'),
  advisory('legacy_retirement_guard_suite', true, 'Baseline memory, amputation, accessibility, and visual parity guards are wired.'),
];
const blockingItems = checks.filter((check) => check.required && !check.ok).map((check) => `${check.id}: ${check.detail}`);
const ready = blockingItems.length === 0;
const report = {
  ok: strict ? ready : true,
  type: 'shell2_graduation_status_report',
  revision: currentRevision(ROOT),
  ready_to_retire_legacy: ready,
  checks,
  blocking_items: blockingItems,
  next_actions: nextActions(checks),
};

writeTextArtifact(outMarkdown, markdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
