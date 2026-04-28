#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SERVICE = 'client/runtime/systems/ui/infring_static/js/shell/dragbar_shell_services.ts';
const DEFAULT_APP = 'client/runtime/systems/ui/infring_static/js/app.ts';
const DEFAULT_DRAG_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part05.ts';
const DEFAULT_CHAT_MAP_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part06.ts';
const DEFAULT_SIDEBAR_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part07.ts';
const DEFAULT_SIDEBAR_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_rail_shell_svelte_source.ts';
const DEFAULT_SIDEBAR_AGENT_LIST_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_agent_list_shell_svelte_source.ts';
const DEFAULT_CHAT_MAP_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_shell_svelte_source.ts';
const DEFAULT_SIDEBAR_BUNDLE = 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_rail_shell.bundle.ts';
const DEFAULT_SIDEBAR_AGENT_LIST_BUNDLE = 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_agent_list_shell.bundle.ts';
const DEFAULT_CHAT_MAP_BUNDLE = 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_shell.bundle.ts';
const DEFAULT_ROUTER = 'adapters/runtime/dashboard_asset_router.ts';
const DEFAULT_HTML_FILES = [
  'client/runtime/systems/ui/infring_static/index_body.html.parts/0001-body-part.part01.html',
  'client/runtime/systems/ui/infring_static/index_body.html.parts/0001-body-part.part01a.html',
  'client/runtime/systems/ui/infring_static/index_body.html.parts/0004-body-part.html',
];
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_dragbar_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_DRAGBAR_OWNERSHIP_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  servicePath: string;
  appPath: string;
  dragPartPath: string;
  chatMapPartPath: string;
  sidebarPartPath: string;
  sidebarSourcePath: string;
  sidebarAgentListSourcePath: string;
  chatMapSourcePath: string;
  sidebarBundlePath: string;
  sidebarAgentListBundlePath: string;
  chatMapBundlePath: string;
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
    appPath: cleanText(readFlag(argv, 'app') || DEFAULT_APP, 400),
    dragPartPath: cleanText(readFlag(argv, 'drag-part') || DEFAULT_DRAG_PART, 400),
    chatMapPartPath: cleanText(readFlag(argv, 'chat-map-part') || DEFAULT_CHAT_MAP_PART, 400),
    sidebarPartPath: cleanText(readFlag(argv, 'sidebar-part') || DEFAULT_SIDEBAR_PART, 400),
    sidebarSourcePath: cleanText(readFlag(argv, 'sidebar-source') || DEFAULT_SIDEBAR_SOURCE, 400),
    sidebarAgentListSourcePath: cleanText(readFlag(argv, 'sidebar-agent-list-source') || DEFAULT_SIDEBAR_AGENT_LIST_SOURCE, 400),
    chatMapSourcePath: cleanText(readFlag(argv, 'chat-map-source') || DEFAULT_CHAT_MAP_SOURCE, 400),
    sidebarBundlePath: cleanText(readFlag(argv, 'sidebar-bundle') || DEFAULT_SIDEBAR_BUNDLE, 400),
    sidebarAgentListBundlePath: cleanText(readFlag(argv, 'sidebar-agent-list-bundle') || DEFAULT_SIDEBAR_AGENT_LIST_BUNDLE, 400),
    chatMapBundlePath: cleanText(readFlag(argv, 'chat-map-bundle') || DEFAULT_CHAT_MAP_BUNDLE, 400),
    routerPath: cleanText(readFlag(argv, 'router') || DEFAULT_ROUTER, 400),
    htmlFiles: htmlOverride ? htmlOverride.split(',').map((value) => cleanText(value, 400)).filter(Boolean) : DEFAULT_HTML_FILES,
  };
}

function readText(path: string): string {
  return readFileSync(resolve(ROOT, path), 'utf8');
}

function requireExists(path: string, violations: Violation[]): boolean {
  if (existsSync(resolve(ROOT, path))) return true;
  violations.push({ kind: 'missing_dragbar_ownership_source', path, detail: 'Required dragbar ownership source is missing.' });
  return false;
}

function requireTokens(path: string, source: string, tokens: string[], kind: string, detail: string): Violation[] {
  return tokens
    .filter((token) => !source.includes(token))
    .map((token) => ({ kind, path, token, detail }));
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Dragbar Ownership Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- checked_sources: ${payload.summary.checked_sources}`);
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
    args.dragPartPath,
    args.chatMapPartPath,
    args.sidebarPartPath,
    args.sidebarSourcePath,
    args.sidebarAgentListSourcePath,
    args.chatMapSourcePath,
    args.sidebarBundlePath,
    args.sidebarAgentListBundlePath,
    args.chatMapBundlePath,
    args.routerPath,
    ...args.htmlFiles,
  ];
  for (const path of paths) requireExists(path, violations);

  if (violations.length === 0) {
    const service = readText(args.servicePath);
    violations.push(...requireTokens(args.servicePath, service, [
      'services.dragbar',
      'normalizeWall: normalizeWall',
      'moveDurationMs: moveDurationMs',
      'hardBounds: hardBounds',
      'softBounds: softBounds',
      'clampWithBounds: clampWithBounds',
      'nearestWall: nearestWall',
      'applyWallLock: applyWallLock',
      'distanceFromWall: distanceFromWall',
      'wallLockOvershoot: wallLockOvershoot',
      'centeredPoint: centeredPoint',
      'wallLockThresholds: wallLockThresholds',
      'resolveWallLock: resolveWallLock',
      'radiusByWall: radiusByWall',
      'lockVisualCssVars: lockVisualCssVars',
      'pulltabStyle: pulltabStyle',
      'shouldIgnoreTarget: shouldIgnoreTarget',
    ], 'missing_dragbar_service_token', 'Dragbar geometry, lock visuals, pulltab placement, and ignore-target behavior must be centralized in the shared Shell dragbar service.'));

    const app = readText(args.appPath);
    violations.push(...requireTokens(args.appPath, app, [
      'dragbarService()',
      'service.hardBounds',
      'service.softBounds',
      'service.clampWithBounds',
      'service.nearestWall',
      'service.normalizeWall',
      'service.applyWallLock',
      'service.distanceFromWall',
      'service.wallLockOvershoot',
      'service.centeredPoint',
      'service.wallLockThresholds',
      'service.resolveWallLock',
      'service.radiusByWall',
      'service.lockVisualCssVars',
      'service.pulltabStyle',
      'service.shouldIgnoreTarget',
      '_chatSidebarDragRenderMaxRows: 10',
      '_chatSidebarDragRowsCache',
    ], 'app_dragbar_wrapper_not_delegated', 'Legacy Alpine-facing wrappers must delegate shared dragbar mechanics while preserving sidebar drag virtualization state.'));

    violations.push(...requireTokens(args.dragPartPath, readText(args.dragPartPath), [
      'service.hardBounds',
      'service.clampWithBounds',
      'service.nearestWall',
      'service.applyWallLock',
      'service.resolveWallLock',
      'service.lockVisualCssVars',
      'service.shouldIgnoreTarget',
    ], 'dragbar_part_not_delegated', 'Segmented drag helper part must mirror shared dragbar service delegation.'));

    violations.push(...requireTokens(args.chatMapPartPath, readText(args.chatMapPartPath), [
      'chatMapHardBounds()',
      'chatMapSetWallLock',
      'chatMapPersistPlacementFromLeft',
      'chatMapPersistPlacementFromTop',
      'startChatMapPointerDrag',
      'handleChatMapPointerMove',
      'endChatMapPointerDrag',
    ], 'chat_map_drag_contract_missing', 'Chat map must keep lock/unlock, anchor persistence, and pointer mechanics connected to parent-owned drag helpers.'));

    violations.push(...requireTokens(args.sidebarPartPath, readText(args.sidebarPartPath), [
      'chatSidebarHardBounds()',
      'chatSidebarSetWallLock',
      'chatSidebarPersistPlacementFromLeft',
      'chatSidebarPersistPlacementFromTop',
      'chatSidebarPulltabStyle',
      'service.pulltabStyle',
      'startChatSidebarPointerDrag',
      'handleChatSidebarPointerMove',
      'endChatSidebarPointerDrag',
      '_chatSidebarDragRowsCache = null',
    ], 'sidebar_drag_contract_missing', 'Chat sidebar must keep lock/unlock, pulltab anchoring, anchor persistence, and drag virtualization cleanup connected to parent-owned drag helpers.'));

    const sidebarSource = readText(args.sidebarSourcePath);
    violations.push(...requireTokens(args.sidebarSourcePath, sidebarSource, [
      "export let dragbarSurface = 'chat-sidebar'",
      'export let parentOwnedMechanics = true',
      '<slot />',
    ], 'svelte_sidebar_dragbar_shell_missing', 'Sidebar rail Svelte shell must explicitly advertise the shared dragbar primitive while preserving slotted legacy markup during migration.'));

    const sidebarAgentListSource = readText(args.sidebarAgentListSourcePath);
    violations.push(...requireTokens(args.sidebarAgentListSourcePath, sidebarAgentListSource, [
      "COMPONENT_TAG = 'infring-sidebar-agent-list-shell'",
      's.sidebarAgents.subscribe',
      'selectAgentChatFromSidebar',
      'startChatSidebarTopologyDrag',
      'handleChatSidebarTopologyDragOver',
      'handleChatSidebarTopologyDrop',
      'archiveAgentFromSidebar',
    ], 'svelte_sidebar_agent_list_shell_missing', 'Sidebar agent rows must be rendered by Svelte while delegating selection, archive, and topology drag actions back to the existing shell methods.'));

    const chatMapSource = readText(args.chatMapSourcePath);
    violations.push(...requireTokens(args.chatMapSourcePath, chatMapSource, [
      "export let dragbarSurface = 'chat-map'",
      'export let parentOwnedMechanics = true',
      'let mapRows = []',
      's.mapRows.subscribe',
      'data-msg-dom-id={row.domId}',
      'startChatMapPointerDrag',
      'stepMessageMap',
      'showMapItemPopup',
      'jumpToMessage',
    ], 'svelte_chat_map_dragbar_shell_missing', 'Chat map Svelte shell must advertise the shared dragbar primitive and own map-row rendering/event delegation.'));

    violations.push(...requireTokens(args.sidebarBundlePath, readText(args.sidebarBundlePath), [
      'dragbarSurface',
      'parentOwnedMechanics',
      'infring-sidebar-rail-shell',
    ], 'stale_sidebar_dragbar_bundle', 'The generated sidebar rail bundle must include the dragbar shell contract props.'));

    violations.push(...requireTokens(args.sidebarAgentListBundlePath, readText(args.sidebarAgentListBundlePath), [
      'infring-sidebar-agent-list-shell',
      'sidebarAgents',
      'selectAgentChatFromSidebar',
      'startChatSidebarTopologyDrag',
      'archiveAgentFromSidebar',
    ], 'stale_sidebar_agent_list_bundle', 'The generated sidebar agent list bundle must include the Svelte row renderer and delegated action hooks.'));

    violations.push(...requireTokens(args.chatMapBundlePath, readText(args.chatMapBundlePath), [
      'dragbarSurface',
      'parentOwnedMechanics',
      'mapRows',
      'chat-map-surface',
      'infring-chat-map-shell',
    ], 'stale_chat_map_dragbar_bundle', 'The generated chat map bundle must include the dragbar shell contract props and Svelte map renderer.'));

    for (const htmlPath of args.htmlFiles) {
      const html = readText(htmlPath);
      if (htmlPath.includes('0001-body-part.part01.html')) {
        violations.push(...requireTokens(htmlPath, html, [
          '<infring-sidebar-rail-shell',
          '<infring-sidebar-agent-list-shell',
          'dragbarsurface="chat-sidebar"',
          'parentownedmechanics="true"',
          'startChatSidebarPointerDrag($event)',
        ], 'sidebar_html_dragbar_shell_not_wired', 'Sidebar host must declare the chat-sidebar dragbar shell and keep parent-owned pointer mechanics.'));
        if (html.includes('<template x-for="agent in chatSidebarVisibleRows"')) {
          violations.push({
            kind: 'sidebar_legacy_alpine_loop_present',
            path: htmlPath,
            token: '<template x-for="agent in chatSidebarVisibleRows"',
            detail: 'Chat sidebar agent rows must not use the retired Alpine visible-row loop.',
          });
        }
      }
      if (htmlPath.includes('0001-body-part.part01a.html')) {
        violations.push(...requireTokens(htmlPath, html, [
          'data-dragbar-pulltab="chat-sidebar"',
          'chatSidebarPulltabStyle()',
          'toggleSidebar()',
        ], 'pulltab_html_dragbar_shell_not_wired', 'Sidebar pulltab must use the shared dragbar pulltab style path while preserving toggle behavior.'));
      }
      if (htmlPath.includes('0004-body-part.html')) {
        violations.push(...requireTokens(htmlPath, html, [
          '<infring-chat-map-shell',
          'dragbarsurface="chat-map"',
          'parentownedmechanics="true"',
        ], 'chat_map_html_dragbar_shell_not_wired', 'Chat map host must declare the chat-map dragbar shell while Svelte owns pointer mechanics.'));
        if (html.includes('<template x-for="(msg, idx) in messages"')) {
          violations.push({
            kind: 'chat_map_legacy_alpine_loop_present',
            path: htmlPath,
            token: '<template x-for="(msg, idx) in messages"',
            detail: 'Chat map rendering must not use the retired Alpine message loop.',
          });
        }
      }
    }

    violations.push(...requireTokens(args.routerPath, readText(args.routerPath), [
      'js/shell/dragbar_shell_services',
      'js/shell/message_metadata_shell_services',
    ], 'dragbar_service_not_loaded', 'The dashboard asset router must load the shared dragbar service before app runtime code.'));
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_dragbar_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      checked_sources: paths.length,
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
