#!/usr/bin/env node
/* eslint-disable no-console */
import vm from 'node:vm';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SERVICE = 'client/runtime/systems/ui/infring_static/js/shell/app_store_shell_services.ts';
const DEFAULT_APP_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/010-core-state.part01.ts';
const DEFAULT_APP_CLOSE_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/020-nav-and-layout.part01a.ts';
const DEFAULT_ROOT_PART = 'client/runtime/systems/ui/infring_static/js/app.ts.parts/030-agent-list-runtime.part01.ts';
const DEFAULT_APP = 'client/runtime/systems/ui/infring_static/js/app.ts';
const DEFAULT_ROUTER = 'adapters/runtime/dashboard_asset_router.ts';
const DEFAULT_SVELTE_SOURCES = [
  'client/runtime/systems/ui/infring_static/js/svelte/bottom_dock_shell_svelte_source.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/dashboard_popup_overlay_shell_svelte_source.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/sidebar_agent_list_shell_svelte_source.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_system_items_shell_svelte_source.ts',
];
const DEFAULT_SVELTE_BUNDLES = [
  'client/runtime/systems/ui/infring_static/js/svelte/bottom_dock_shell.bundle.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/dashboard_popup_overlay_shell.bundle.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/sidebar_agent_list_shell.bundle.ts',
  'client/runtime/systems/ui/infring_static/js/svelte/taskbar_system_items_shell.bundle.ts',
];
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_app_store_bridge_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_APP_STORE_BRIDGE_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  servicePath: string;
  appPartPath: string;
  appClosePartPath: string;
  rootPartPath: string;
  appPath: string;
  routerPath: string;
  svelteSources: string[];
  svelteBundles: string[];
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
    const raw = readFlag(argv, name);
    return raw ? raw.split(',').map((row) => cleanText(row, 400)).filter(Boolean) : fallback.slice();
  };
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    servicePath: cleanText(readFlag(argv, 'service') || DEFAULT_SERVICE, 400),
    appPartPath: cleanText(readFlag(argv, 'app-part') || DEFAULT_APP_PART, 400),
    appClosePartPath: cleanText(readFlag(argv, 'app-close-part') || DEFAULT_APP_CLOSE_PART, 400),
    rootPartPath: cleanText(readFlag(argv, 'root-part') || DEFAULT_ROOT_PART, 400),
    appPath: cleanText(readFlag(argv, 'app') || DEFAULT_APP, 400),
    routerPath: cleanText(readFlag(argv, 'router') || DEFAULT_ROUTER, 400),
    svelteSources: splitFlag('svelte-sources', DEFAULT_SVELTE_SOURCES),
    svelteBundles: splitFlag('svelte-bundles', DEFAULT_SVELTE_BUNDLES),
  };
}

function readText(path: string): string {
  return readFileSync(resolve(ROOT, path), 'utf8');
}

function requireExists(path: string, violations: Violation[]): boolean {
  if (existsSync(resolve(ROOT, path))) return true;
  violations.push({ kind: 'missing_app_store_bridge_source', path, detail: 'Required Shell app-store bridge source is missing.' });
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

function smokeService(path: string): Violation[] {
  const violations: Violation[] = [];
  const source = readText(path);
  const events: any[] = [];
  const windowStub: any = {
    location: { hash: '#agents' },
    InfringApp: null,
    addEventListener() {},
    dispatchEvent(event: any) { events.push(event); },
    CustomEvent: function CustomEvent(type: string, init: any) {
      this.type = type;
      this.detail = init && init.detail;
    },
  };
  const sandbox: any = {
    window: windowStub,
    CustomEvent: windowStub.CustomEvent,
    console,
  };
  vm.createContext(sandbox);
  try {
    vm.runInContext(source, sandbox, { filename: path });
    const service = sandbox.window.InfringSharedShellServices &&
      sandbox.window.InfringSharedShellServices.appStore;
    if (!service || typeof service.subscribe !== 'function' || typeof service.snapshot !== 'function') {
      violations.push({ kind: 'app_store_bridge_smoke_failed', path, detail: 'Bridge did not expose Svelte-compatible subscribe/snapshot methods.' });
      return violations;
    }
    let observed: any = null;
    const unsubscribe = service.subscribe((state: any) => { observed = state; });
    const runtime = {
      stores: {} as Record<string, any>,
      store(name: string, value?: any) {
        if (arguments.length > 1) this.stores[name] = value;
        return this.stores[name];
      },
    };
    const registered = service.registerAlpineStore(runtime, 'app', {
      agents: [{ id: 'agent-a' }],
      agentCount: 1,
      activeAgentId: 'agent-a',
      theme: 'dark',
      themeMode: 'system',
      connectionState: 'connected',
    });
    service.registerShellRoot({ page: 'chat', theme: 'light' });
    const snapshot = service.snapshot();
    service.set('activeAgentId', 'agent-b');
    service.assign({ connectionState: 'reconnecting' });
    if (typeof unsubscribe === 'function') unsubscribe();
    if (!registered || runtime.stores.app !== registered) {
      violations.push({ kind: 'app_store_bridge_registration_failed', path, detail: 'Bridge did not register and return the compatibility app store.' });
    }
    if (snapshot.page !== 'chat' || snapshot.agents.length !== 1 || snapshot.activeAgentId !== 'agent-a') {
      violations.push({ kind: 'app_store_bridge_snapshot_failed', path, detail: 'Bridge snapshot did not expose page, agent list, and active agent state.' });
    }
    if (registered.activeAgentId !== 'agent-b' || registered.connectionState !== 'reconnecting') {
      violations.push({ kind: 'app_store_bridge_mutation_failed', path, detail: 'Bridge set/assign compatibility mutation helpers did not update the backing store.' });
    }
    if (!observed || events.length === 0) {
      violations.push({ kind: 'app_store_bridge_subscription_failed', path, detail: 'Bridge did not notify subscribers and dispatch change events.' });
    }
  } catch (error: any) {
    violations.push({ kind: 'app_store_bridge_smoke_exception', path, detail: String(error && error.message || error) });
  }
  return violations;
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell App Store Bridge Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- checked_sources: ${payload.summary.checked_sources}`);
  lines.push(`- svelte_callers: ${payload.summary.svelte_callers}`);
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
    args.appPartPath,
    args.appClosePartPath,
    args.rootPartPath,
    args.appPath,
    args.routerPath,
    ...args.svelteSources,
    ...args.svelteBundles,
  ];
  for (const path of paths) requireExists(path, violations);

  if (violations.length === 0) {
    const service = readText(args.servicePath);
    violations.push(...requireTokens(args.servicePath, service, [
      'services.appStore',
      'current: current',
      'root: root',
      'snapshot: snapshot',
      'subscribe: subscribe',
      'notify: emit',
      'registerSource: registerSource',
      'registerShellRoot: registerShellRoot',
      'registerAlpineStore: registerAlpineStore',
      'set: set',
      'assign: assign',
      'method: method',
      'page: page',
      'route: page',
      'theme: theme',
      'themeMode: themeMode',
      'agents: agents',
      'activeAgentId',
    ], 'missing_app_store_bridge_service_token', 'Shell route/page/theme/agent state must be centralized in a Svelte-compatible app-store bridge.'));
    violations.push(...smokeService(args.servicePath));

    violations.push(...requireTokens(args.routerPath, readText(args.routerPath), [
      "readForkScript(staticDir, 'js/shell/app_store_shell_services')",
      "readForkScript(staticDir, 'js/svelte/taskbar_system_items_shell.bundle')",
      "readForkScript(staticDir, 'js/svelte/bottom_dock_shell.bundle')",
    ], 'app_store_bridge_router_not_loaded', 'Dashboard asset router must load the app-store bridge before active Svelte shell callers.'));

    violations.push(...requireTokens(args.appPartPath, readText(args.appPartPath), [
      'function infringShellAppStoreBridge()',
      'function infringShellAppStoreCurrent()',
      'var appStoreDefinition = {',
      ': infringShellAppStoreCurrent()',
    ], 'app_store_bootstrap_not_bridged', 'The temporary Alpine compatibility store must be defined through the Shell app-store bridge seam.'));
    violations.push(...forbidTokens(args.appPartPath, readText(args.appPartPath), [
      "Alpine.store('app',",
      'Alpine.store("app",',
    ], 'app_store_direct_bootstrap_returned', 'The app store bootstrap must not directly own Alpine.store registration.'));
    violations.push(...requireTokens(args.appClosePartPath, readText(args.appClosePartPath), [
      'appStoreBridge.registerAlpineStore(Alpine,',
      "alpineRuntime.store('app', appStoreDefinition)",
      'window.InfringApp = alpineRuntime.store',
    ], 'app_store_close_part_not_registered', 'The app store definition must be registered through the bridge, with a legacy fallback only through a local runtime variable.'));
    violations.push(...requireTokens(args.rootPartPath, readText(args.rootPartPath), [
      'appStoreBridge.registerShellRoot(this)',
      "self.notifyShellAppStore('route_changed')",
      "this.notifyShellAppStore('navigate')",
      'shellAppStoreBridge()',
      'notifyShellAppStore(reason)',
      'bridge.current()',
    ], 'shell_root_not_registered_with_app_store_bridge', 'The root shell page object must register page/theme state and route changes with the app-store bridge.'));

    for (const path of args.svelteSources) {
      const source = readText(path);
      violations.push(...requireTokens(path, source, [
        'function appStoreService()',
        'services.appStore',
        'service.current',
      ], 'svelte_app_store_caller_not_bridged', 'Active Svelte shell callers must read app state through services.appStore.current().'));
      violations.push(...forbidTokens(path, source, [
        "window.Alpine.store('app')",
        'window.Alpine.store("app")',
        'window.InfringApp',
      ], 'svelte_direct_app_store_access_returned', 'Svelte shell callers must not directly read Alpine or window.InfringApp.'));
    }
    for (const path of args.svelteBundles) {
      const bundle = readText(path);
      violations.push(...requireTokens(path, bundle, ['appStore', 'current'], 'stale_app_store_bridge_bundle', 'Generated Svelte bundle is stale or missing app-store bridge calls.'));
      violations.push(...forbidTokens(path, bundle, ['Alpine.store("app")', "Alpine.store('app')"], 'stale_direct_alpine_store_bundle', 'Generated Svelte bundle still contains direct Alpine app-store access.'));
    }
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_app_store_bridge_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      checked_sources: Array.from(new Set(paths)).length,
      svelte_callers: args.svelteSources.length,
      violations: violations.length,
    },
    violations,
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: args.outJson });
  if (args.strict && violations.length) process.exitCode = 1;
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
