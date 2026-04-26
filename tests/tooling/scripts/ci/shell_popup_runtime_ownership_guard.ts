#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SHARED_SERVICE = 'client/runtime/systems/ui/infring_static/js/shell/shared_shell_services.ts';
const DEFAULT_APP = 'client/runtime/systems/ui/infring_static/js/app.ts';
const DEFAULT_PART10 = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part10.ts';
const DEFAULT_PART11 = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part11.ts';
const DEFAULT_PART02 = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/040-events-and-actions.part02.ts';
const DEFAULT_CONTRACTS = 'client/runtime/systems/ui/infring_static/js/svelte/svelte_shell_contracts.json';
const DEFAULT_SVELTE_DIR = 'client/runtime/systems/ui/infring_static/js/svelte';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_popup_runtime_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_POPUP_RUNTIME_OWNERSHIP_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  servicePath: string;
  appPath: string;
  part10Path: string;
  part11Path: string;
  part02Path: string;
  contractsPath: string;
  svelteDir: string;
  outJson: string;
  outMarkdown: string;
};

type SourceSpec = {
  path: string;
  methods: Record<string, string[]>;
};

type Violation = {
  kind: string;
  path?: string;
  method?: string;
  token?: string;
  detail: string;
};

const SERVICE_EXPORT_TOKENS = [
  'services.popup',
  'normalizeSide: normalizePopupSide',
  'oppositeSide: oppositePopupSide',
  'wallAffinity: popupWallAffinity',
  'sideAwayFromNearestWall: popupSideAwayFromNearestWall',
  'horizontalAwayFromNearestWall: popupHorizontalAwayFromNearestWall',
  'verticalAwayFromNearestWall: popupVerticalAwayFromNearestWall',
  'axisAwareSideAway: popupAxisAwareSideAway',
  'anchorPoint: popupAnchorPoint',
  'dropdownClass: popupDropdownClass',
  'emptyState: emptyPopupState',
  'origin: popupOrigin',
  'openState: openPopupState',
  'closeState: closePopupState',
  'stateOrigin: popupStateOrigin',
  'overlayClass: popupOverlayClass',
  'overlayStyle: popupOverlayStyle',
];

const REQUIRED_SVELTE_SHELLS = [
  'infring-popup-window-shell',
  'infring-taskbar-menu-shell',
  'infring-taskbar-dropdown-cluster-shell',
  'infring-taskbar-hero-menu-shell',
  'infring-model-picker-menu-shell',
  'infring-git-tree-picker-shell',
  'infring-slash-command-menu-shell',
];

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    servicePath: cleanText(readFlag(argv, 'service') || DEFAULT_SHARED_SERVICE, 400),
    appPath: cleanText(readFlag(argv, 'app') || DEFAULT_APP, 400),
    part10Path: cleanText(readFlag(argv, 'part10') || DEFAULT_PART10, 400),
    part11Path: cleanText(readFlag(argv, 'part11') || DEFAULT_PART11, 400),
    part02Path: cleanText(readFlag(argv, 'part02') || DEFAULT_PART02, 400),
    contractsPath: cleanText(readFlag(argv, 'contracts') || DEFAULT_CONTRACTS, 400),
    svelteDir: cleanText(readFlag(argv, 'svelte-dir') || DEFAULT_SVELTE_DIR, 400),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function readText(path: string): string {
  return readFileSync(resolve(ROOT, path), 'utf8');
}

function readJson(path: string): any {
  return JSON.parse(readText(path));
}

function methodBodies(source: string, methodName: string): string[] {
  const bodies: string[] = [];
  const matcher = new RegExp(`${methodName}\\s*\\([^)]*\\)\\s*\\{`, 'g');
  let match = matcher.exec(source);
  while (match) {
    let depth = 1;
    let cursor = matcher.lastIndex;
    while (cursor < source.length && depth > 0) {
      const char = source[cursor];
      if (char === '{') depth += 1;
      if (char === '}') depth -= 1;
      cursor += 1;
    }
    bodies.push(source.slice(match.index, cursor));
    match = matcher.exec(source);
  }
  return bodies;
}

function validateSharedService(path: string, source: string): Violation[] {
  const violations: Violation[] = [];
  for (const token of SERVICE_EXPORT_TOKENS) {
    if (!source.includes(token)) {
      violations.push({
        kind: 'missing_shared_popup_service_token',
        path,
        token,
        detail: 'Popup, dropdown, and menu behavior must be owned by InfringSharedShellServices.popup.',
      });
    }
  }
  return violations;
}

function validateMethodDelegates(spec: SourceSpec): Violation[] {
  const source = readText(spec.path);
  const violations: Violation[] = [];
  for (const [method, tokens] of Object.entries(spec.methods)) {
    const bodies = methodBodies(source, method);
    if (!bodies.length) {
      violations.push({ kind: 'missing_popup_compat_method', path: spec.path, method, detail: 'Expected Alpine compatibility wrapper was not found.' });
      continue;
    }
    bodies.forEach((body, index) => {
      for (const token of tokens) {
        if (!body.includes(token)) {
          violations.push({
            kind: 'popup_runtime_not_delegated',
            path: spec.path,
            method: bodies.length > 1 ? `${method}#${index + 1}` : method,
            token,
            detail: 'Alpine-facing popup helper must delegate runtime behavior to shared shell services.',
          });
        }
      }
    });
  }
  return violations;
}

function validateSvelteShells(args: Args): Violation[] {
  const violations: Violation[] = [];
  const contracts = readJson(args.contractsPath).contracts || {};
  for (const tag of REQUIRED_SVELTE_SHELLS) {
    if (!Object.prototype.hasOwnProperty.call(contracts, tag)) {
      violations.push({
        kind: 'missing_svelte_popup_shell_contract',
        path: args.contractsPath,
        token: tag,
        detail: 'Popup/menu shell must remain declared as a Svelte migration seam.',
      });
    }
  }
  const sourceMap: Record<string, string> = {
    'infring-popup-window-shell': 'popup_window_shell_svelte_source.ts',
    'infring-taskbar-menu-shell': 'taskbar_menu_shell_svelte_source.ts',
    'infring-taskbar-dropdown-cluster-shell': 'taskbar_dropdown_cluster_shell_svelte_source.ts',
    'infring-taskbar-hero-menu-shell': 'taskbar_hero_menu_shell_svelte_source.ts',
    'infring-model-picker-menu-shell': 'model_picker_menu_shell_svelte_source.ts',
    'infring-git-tree-picker-shell': 'git_tree_picker_shell_svelte_source.ts',
    'infring-slash-command-menu-shell': 'slash_command_menu_shell_svelte_source.ts',
  };
  for (const [tag, file] of Object.entries(sourceMap)) {
    const path = `${args.svelteDir}/${file}`;
    if (!existsSync(resolve(ROOT, path))) {
      violations.push({
        kind: 'missing_svelte_popup_shell_source',
        path,
        token: tag,
        detail: 'Popup/menu shell source must exist before Alpine behavior can retire.',
      });
    }
  }
  return violations;
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Popup Runtime Ownership Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- checked_sources: ${payload.summary.checked_sources}`);
  lines.push(`- checked_methods: ${payload.summary.checked_methods}`);
  lines.push(`- svelte_shells: ${payload.summary.svelte_shells}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.path || 'unknown'} ${violation.method || ''} ${violation.token || ''}`);
  }
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = readArgs(argv);
  const serviceSource = readText(args.servicePath);
  const sources: SourceSpec[] = [
    {
      path: args.appPath,
      methods: {
        dashboardPopupService: ['InfringSharedShellServices', '.popup'],
        clearDashboardPopupState: ['service.emptyState'],
        normalizeDashboardPopupSide: ['service.normalizeSide'],
        dashboardOppositeSide: ['service.oppositeSide'],
        dashboardPopupWallAffinity: ['service.wallAffinity'],
        dashboardPopupSideAwayFromNearestWall: ['service.sideAwayFromNearestWall'],
        dashboardPopupHorizontalAwayFromNearestWall: ['service.horizontalAwayFromNearestWall'],
        dashboardPopupVerticalAwayFromNearestWall: ['service.verticalAwayFromNearestWall'],
        dashboardPopupAxisAwareSideAway: ['service.axisAwareSideAway'],
        taskbarAnchoredDropdownClass: ['service.dropdownClass'],
        dashboardPopupAnchorPoint: ['service.anchorPoint'],
        showDashboardPopup: ['service.openState'],
        hideDashboardPopup: ['service.closeState'],
        dashboardPopupOrigin: ['service.origin'],
        dashboardPopupStateOrigin: ['service.stateOrigin'],
        dashboardPopupOverlayClass: ['service.overlayClass', 'fogged-glass'],
        dashboardPopupOverlayStyle: ['service.overlayStyle'],
      },
    },
    {
      path: args.part10Path,
      methods: {
        dashboardPopupService: ['InfringSharedShellServices', '.popup'],
        clearDashboardPopupState: ['service.emptyState'],
        normalizeDashboardPopupSide: ['service.normalizeSide'],
        dashboardOppositeSide: ['service.oppositeSide'],
        dashboardPopupWallAffinity: ['service.wallAffinity'],
        dashboardPopupSideAwayFromNearestWall: ['service.sideAwayFromNearestWall'],
        dashboardPopupHorizontalAwayFromNearestWall: ['service.horizontalAwayFromNearestWall'],
        dashboardPopupVerticalAwayFromNearestWall: ['service.verticalAwayFromNearestWall'],
        dashboardPopupAxisAwareSideAway: ['service.axisAwareSideAway'],
        taskbarAnchoredDropdownClass: ['service.dropdownClass'],
        dashboardPopupAnchorPoint: ['service.anchorPoint'],
        showDashboardPopup: ['service.openState'],
        hideDashboardPopup: ['service.closeState'],
      },
    },
    {
      path: args.part11Path,
      methods: {
        dashboardPopupOrigin: ['service.origin'],
        dashboardPopupStateOrigin: ['service.stateOrigin'],
        dashboardPopupOverlayClass: ['service.overlayClass', 'fogged-glass'],
        dashboardPopupOverlayStyle: ['service.overlayStyle'],
      },
    },
    {
      path: args.part02Path,
      methods: {
        showDashboardPopup: ['service.openState'],
        hideDashboardPopup: ['service.closeState'],
      },
    },
  ];
  const violations = [
    ...validateSharedService(args.servicePath, serviceSource),
    ...sources.flatMap((source) => validateMethodDelegates(source)),
    ...validateSvelteShells(args),
  ];
  const checkedMethods = sources.reduce((sum, source) => sum + Object.keys(source.methods).length, 0);
  const payload = {
    ok: violations.length === 0 || !args.strict,
    type: 'shell_popup_runtime_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: args,
    summary: {
      checked_sources: sources.length + 1,
      checked_methods: checkedMethods,
      svelte_shells: REQUIRED_SVELTE_SHELLS.length,
      violations: violations.length,
    },
    violations,
  };
  emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
    history: false,
    stdout: false,
  });
  writeTextArtifact(args.outMarkdown, markdown(payload));
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  return payload.ok ? 0 : 1;
}

if (require.main === module) {
  run().then((code) => process.exit(code));
}

module.exports = { run };
