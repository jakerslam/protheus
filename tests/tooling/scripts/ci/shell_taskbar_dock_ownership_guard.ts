#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SERVICE = 'client/runtime/systems/ui/infring_static/js/shell/taskbar_dock_shell_services.ts';
const DEFAULT_APP = 'client/runtime/systems/ui/infring_static/js/app.ts';
const DEFAULT_LAYOUT_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part01a.ts';
const DEFAULT_STATE_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part02.ts';
const DEFAULT_DOCK_STATE_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part02a.ts';
const DEFAULT_CONTAINMENT_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part03.ts';
const DEFAULT_TASKBAR_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part05.ts';
const DEFAULT_DOCK_ORDER_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part07.ts';
const DEFAULT_DISPLAY_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/030-agent-list-runtime.part02.ts';
const DEFAULT_ROUTER = 'adapters/runtime/dashboard_asset_router.ts';
const DEFAULT_SVELTE_SOURCES = [
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_menu_shell_svelte_source.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_hero_menu_shell_svelte_source.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_nav_cluster_shell_svelte_source.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_dropdown_cluster_shell_svelte_source.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_system_items_shell_svelte_source.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/bottom_dock_shell_svelte_source.ts',
];
const DEFAULT_SVELTE_BUNDLES = [
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_menu_shell.bundle.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_hero_menu_shell.bundle.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_nav_cluster_shell.bundle.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_dropdown_cluster_shell.bundle.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_system_items_shell.bundle.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/bottom_dock_shell.bundle.ts',
];
const DEFAULT_HTML_FILES = [
  'client/runtime/systems/ui/infring_static/index_body.html.parts/0001-body-part.part01a.html',
  'client/runtime/systems/ui/infring_static/index_body.html.parts/0001-body-part.part02.html',
  'client/runtime/systems/ui/infring_static/index_body.html.parts/0001-body-part.part03.html',
];
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_taskbar_dock_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_TASKBAR_DOCK_OWNERSHIP_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  servicePath: string;
  appPath: string;
  layoutPartPath: string;
  statePartPath: string;
  dockStatePartPath: string;
  containmentPartPath: string;
  taskbarPartPath: string;
  dockOrderPartPath: string;
  displayPartPath: string;
  routerPath: string;
  svelteSources: string[];
  svelteBundles: string[];
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
  const splitFlag = (name: string, fallback: string[]) => {
    const value = readFlag(argv, name);
    return value ? value.split(',').map((row) => cleanText(row, 400)).filter(Boolean) : fallback.slice();
  };
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    servicePath: cleanText(readFlag(argv, 'service') || DEFAULT_SERVICE, 400),
    appPath: cleanText(readFlag(argv, 'app') || DEFAULT_APP, 400),
    layoutPartPath: cleanText(readFlag(argv, 'layout-part') || DEFAULT_LAYOUT_PART, 400),
    statePartPath: cleanText(readFlag(argv, 'state-part') || DEFAULT_STATE_PART, 400),
    dockStatePartPath: cleanText(readFlag(argv, 'dock-state-part') || DEFAULT_DOCK_STATE_PART, 400),
    containmentPartPath: cleanText(readFlag(argv, 'containment-part') || DEFAULT_CONTAINMENT_PART, 400),
    taskbarPartPath: cleanText(readFlag(argv, 'taskbar-part') || DEFAULT_TASKBAR_PART, 400),
    dockOrderPartPath: cleanText(readFlag(argv, 'dock-order-part') || DEFAULT_DOCK_ORDER_PART, 400),
    displayPartPath: cleanText(readFlag(argv, 'display-part') || DEFAULT_DISPLAY_PART, 400),
    routerPath: cleanText(readFlag(argv, 'router') || DEFAULT_ROUTER, 400),
    svelteSources: splitFlag('svelte-sources', DEFAULT_SVELTE_SOURCES),
    svelteBundles: splitFlag('svelte-bundles', DEFAULT_SVELTE_BUNDLES),
    htmlFiles: splitFlag('html-files', DEFAULT_HTML_FILES),
  };
}

function readText(path: string): string {
  return readFileSync(resolve(ROOT, path), 'utf8');
}

function requireExists(path: string, violations: Violation[]): boolean {
  if (existsSync(resolve(ROOT, path))) return true;
  violations.push({ kind: 'missing_taskbar_dock_ownership_source', path, detail: 'Required taskbar/dock ownership source is missing.' });
  return false;
}

function requireTokens(path: string, source: string, tokens: string[], kind: string, detail: string): Violation[] {
  return tokens
    .filter((token) => !source.includes(token))
    .map((token) => ({ kind, path, token, detail }));
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Taskbar Dock Ownership Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- checked_sources: ${payload.summary.checked_sources}`);
  lines.push(`- svelte_shells: ${payload.summary.svelte_shells}`);
  lines.push(`- html_shells: ${payload.summary.html_shells}`);
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
    args.appPath,
    args.layoutPartPath,
    args.statePartPath,
    args.dockStatePartPath,
    args.containmentPartPath,
    args.taskbarPartPath,
    args.dockOrderPartPath,
    args.displayPartPath,
    args.routerPath,
    ...args.svelteSources,
    ...args.svelteBundles,
    ...args.htmlFiles,
  ];
  for (const path of paths) requireExists(path, violations);

  if (violations.length === 0) {
    const service = readText(args.servicePath);
    violations.push(...requireTokens(args.servicePath, service, [
      'services.taskbarDock',
      'defaultProfile: defaultProfile',
      'defaultLayoutConfig: defaultLayoutConfig',
      'seedLayoutConfig: seedLayoutConfig',
      'readDisplayBackground: readDisplayBackground',
      'writeDisplayBackground: writeDisplayBackground',
      'normalizeTaskbarEdge: normalizeTaskbarEdge',
      'readTaskbarOrder: readTaskbarOrder',
      'persistTaskbarOrder: persistTaskbarOrder',
      'readDockOrder: readDockOrder',
      'persistDockOrder: persistDockOrder',
      'dockTaskbarContained: dockTaskbarContained',
      'dockTaskbarContainedAnchorX: dockTaskbarContainedAnchorX',
      'dockTaskbarContainedMetrics: dockTaskbarContainedMetrics',
      'taskbarContainerStyle: taskbarContainerStyle',
    ], 'missing_taskbar_dock_service_token', 'Taskbar/dock layout, persistence, containment, and display-config reads must be centralized in the shared Shell taskbarDock service.'));

    const app = readText(args.appPath);
    violations.push(...requireTokens(args.appPath, app, [
      'infringTaskbarDockService()',
      'taskbarDockService()',
      'service.readLayoutConfig',
      'service.readDisplayBackground',
      'service.readTaskbarOrder',
      'service.readDockOrder',
      'service.taskbarContainerStyle',
      'service.dockTaskbarContained',
      'service.dockTaskbarContainedAnchorX',
      'service.dockTaskbarContainedMetrics',
      'service.persistTaskbarOrder',
      'service.persistDockOrder',
      'service.normalizeBackgroundTemplate',
    ], 'app_taskbar_dock_wrapper_not_delegated', 'Legacy Alpine-facing taskbar/dock/display wrappers must delegate shared Shell taskbarDock behavior.'));

    violations.push(...requireTokens(args.layoutPartPath, readText(args.layoutPartPath), [
      'infringTaskbarDockService()',
      'service.defaultProfile',
      'service.defaultLayoutConfig',
      'service.readLayoutConfig',
      'service.seedLayoutConfig',
      'service.readDisplayBackground',
    ], 'layout_part_not_delegated', 'Shell layout config and first-run OS defaults must be sourced through the shared taskbarDock service.'));

    violations.push(...requireTokens(args.statePartPath, readText(args.statePartPath), [
      'taskbarDockService()',
      'service.readLayoutConfig().taskbar.edge',
      "service.readTaskbarOrder('left')",
      "service.readTaskbarOrder('right')",
    ], 'taskbar_state_part_not_delegated', 'Taskbar edge and reorder initial state must read from shared Shell taskbarDock config.'));

    violations.push(...requireTokens(args.dockStatePartPath, readText(args.dockStatePartPath), [
      'service.readDockOrder()',
      'service.dockTileConfig()',
      'service.readLayoutConfig().dock.placement',
      'service.readLayoutConfig().dock.wallLock',
    ], 'dock_state_part_not_delegated', 'Dock order, tile registry, placement, and wall lock initial state must read from shared Shell taskbarDock config.'));

    violations.push(...requireTokens(args.containmentPartPath, readText(args.containmentPartPath), [
      'service.dockTaskbarContained',
      'service.dockTaskbarContainedAnchorX',
      'service.dockTaskbarContainedMetrics',
    ], 'dock_containment_part_not_delegated', 'Dock-in-taskbar containment must use shared Shell taskbarDock geometry helpers.'));

    violations.push(...requireTokens(args.taskbarPartPath, readText(args.taskbarPartPath), [
      'service.normalizeTaskbarEdge',
      'service.taskbarContainerStyle',
      'service.shouldIgnoreTarget',
      'service.taskbarOrderDefaults',
      'service.taskbarStorageKey',
      'service.normalizeOrder',
      'service.persistTaskbarOrder',
      'service.orderIndex',
    ], 'taskbar_part_not_delegated', 'Taskbar drag/persistence/reorder wrappers must delegate to shared Shell services.'));

    violations.push(...requireTokens(args.dockOrderPartPath, readText(args.dockOrderPartPath), [
      'service.dockDefaultOrder',
      'service.dockSlotStyle',
      'service.normalizeOrder',
      'service.persistDockOrder',
      'service.orderIndex',
    ], 'dock_order_part_not_delegated', 'Dock tile ordering and slot style must delegate to the shared Shell taskbarDock service.'));

    violations.push(...requireTokens(args.displayPartPath, readText(args.displayPartPath), [
      'service.normalizeBackgroundTemplate',
      'service.writeDisplayBackground',
    ], 'display_part_not_delegated', 'Display background template reads/writes must delegate to the shared Shell taskbarDock service.'));

    for (const sourcePath of args.svelteSources) {
      const sourceText = readText(sourcePath);
      const structuralTokens = [
        "export let shellPrimitive = 'taskbar-dock'",
        'export let parentOwnedMechanics = true',
      ];
      const renderTokens = sourcePath.includes('bottom_dock_shell')
        ? [
            'infring-bottom-dock-shell',
            'bottomDockContainerStyle',
            'bottomDockSlotStyle',
            'bottomDockTileStyle',
            'setBottomDockHover',
            'clearBottomDockHover',
            'startBottomDockPointerDrag',
          ]
        : sourcePath.includes('taskbar_system_items_shell')
          ? [
              'infring-taskbar-system-items-shell',
              'normalizeTaskbarReorder',
              'taskbarReorderItemStyle',
              'handleTaskbarReorderDragStart',
              'toggleNotifications',
              'taskbarClockMainLabel',
            ]
          : ['<slot />'];
      violations.push(...requireTokens(sourcePath, sourceText, [
        ...structuralTokens,
        ...renderTokens,
      ], 'svelte_taskbar_dock_shell_missing', 'Taskbar Svelte shell seams must advertise the shared taskbarDock primitive while preserving slotted legacy markup or Svelte-owned rendering delegates.'));
    }

    for (const bundlePath of args.svelteBundles) {
      violations.push(...requireTokens(bundlePath, readText(bundlePath), [
        'shellPrimitive',
        'parentOwnedMechanics',
      ], 'stale_taskbar_dock_bundle', 'Generated taskbar bundles must include the taskbarDock shell contract props.'));
    }

    for (const htmlPath of args.htmlFiles) {
      const html = readText(htmlPath);
      if (htmlPath.includes('part01a')) {
        violations.push(...requireTokens(htmlPath, html, [
          'data-shell-primitive="taskbar-dock"',
          'data-taskbar-dock-surface="global-taskbar"',
          'shellprimitive="taskbar-dock"',
          'wrapperrole="taskbar-hero"',
          'wrapperrole="taskbar-nav"',
          'wrapperrole="taskbar-dropdowns"',
          'parentownedmechanics="true"',
        ], 'taskbar_html_shell_not_wired', 'Taskbar host and left wrappers must declare taskbarDock shell primitive ownership.'));
      }
      if (htmlPath.includes('part02')) {
        violations.push(...requireTokens(htmlPath, html, [
          'infring-taskbar-system-items-shell',
          'shellprimitive="taskbar-dock"',
          'wrapperrole="taskbar-system-items"',
          'parentownedmechanics="true"',
        ], 'taskbar_menu_html_shell_not_wired', 'Taskbar system items must declare taskbarDock shell primitive ownership.'));
      }
      if (htmlPath.includes('part03')) {
        violations.push(...requireTokens(htmlPath, html, [
          'infring-bottom-dock-shell',
          'shellprimitive="taskbar-dock"',
          'parentownedmechanics="true"',
        ], 'dock_html_shell_not_wired', 'Dock host must declare taskbarDock containment ownership while preserving existing tile mechanics.'));
      }
    }

    violations.push(...requireTokens(args.routerPath, readText(args.routerPath), [
      'js/shell/dragbar_shell_services',
      'js/shell/taskbar_dock_shell_services',
      'js/shell/message_metadata_shell_services',
      'js/svelte/taskbar_system_items_shell.bundle',
      'js/svelte/bottom_dock_shell.bundle',
    ], 'taskbar_dock_service_not_loaded', 'The dashboard asset router must load the shared taskbarDock service between dragbar primitives and app runtime code.'));
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_taskbar_dock_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      checked_sources: paths.length,
      svelte_shells: args.svelteSources.length,
      html_shells: args.htmlFiles.length,
      violations: violations.length,
    },
    violations,
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  const exitCode = emitStructuredResult(payload, { outPath: args.outJson, strict: args.strict, ok: payload.ok });
  if (exitCode) process.exitCode = exitCode;
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
