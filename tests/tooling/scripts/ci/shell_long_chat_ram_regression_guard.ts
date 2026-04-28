#!/usr/bin/env node
/* eslint-disable no-console */
import vm from 'node:vm';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CHAT_STORE = 'client/runtime/systems/ui/infring_static/js/chat_store.ts';
const DEFAULT_CHAT_PAGE_PART = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/120-slash-and-agent-select.ts';
const DEFAULT_THREAD_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/chat_thread_shell_svelte_source.ts';
const DEFAULT_PLACEHOLDER_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/message_placeholder_shell_svelte_source.ts';
const DEFAULT_BUBBLE_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/chat_bubble_svelte_source.ts';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_long_chat_ram_regression_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_LONG_CHAT_RAM_REGRESSION_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  chatStorePath: string;
  chatPagePartPath: string;
  threadSourcePath: string;
  placeholderSourcePath: string;
  bubbleSourcePath: string;
  messageCount: number;
  maxRenderedThreadMessages: number;
  maxDomNodesAfterOpen: number;
  maxHeapGrowthMb: number;
};

type Violation = {
  kind: string;
  path?: string;
  token?: string;
  detail: string;
  observed?: number;
  limit?: number;
};

type MemorySnapshot = {
  label: string;
  message_count: number;
  projected_thread_messages: number;
  map_rows: number;
  heap: {
    heap_unsupported: false;
    used_js_heap_mb: number;
    total_js_heap_mb: number;
  };
  dom_counts: {
    total_nodes: number;
    scripts: number;
    styles: number;
    divs: number;
  };
  custom_element_counts: Record<string, number>;
};

type ProjectionSmoke = {
  ok: boolean;
  before_projected_messages: number;
  before_map_rows: number;
  after_open_projected_messages: number;
  after_open_map_rows: number;
  centered_projected_messages: number;
  tail_projected_messages: number;
};

const SYNTHETIC_BUDGET = {
  baseDomNodes: 1600,
  shellNodesAfterOpen: 220,
  domNodesPerProjectedMessage: 38,
  domNodesPerMapRow: 5,
  baseHeapMb: 96,
  shellHeapAfterOpenMb: 8,
  heapKbPerRetainedMessage: 16,
  heapKbPerProjectedMessage: 560,
  heapKbPerMapRow: 3,
};

function numberFlag(argv: string[], name: string, fallback: number, min: number, max: number): number {
  const raw = readFlag(argv, name);
  const parsed = Number(raw == null || raw === '' ? fallback : raw);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(min, Math.min(max, Math.round(parsed)));
}

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    chatStorePath: cleanText(readFlag(argv, 'chat-store') || DEFAULT_CHAT_STORE, 400),
    chatPagePartPath: cleanText(readFlag(argv, 'chat-page-part') || DEFAULT_CHAT_PAGE_PART, 400),
    threadSourcePath: cleanText(readFlag(argv, 'thread-source') || DEFAULT_THREAD_SOURCE, 400),
    placeholderSourcePath: cleanText(readFlag(argv, 'placeholder-source') || DEFAULT_PLACEHOLDER_SOURCE, 400),
    bubbleSourcePath: cleanText(readFlag(argv, 'bubble-source') || DEFAULT_BUBBLE_SOURCE, 400),
    messageCount: numberFlag(argv, 'message-count', 1000, 100, 20000),
    maxRenderedThreadMessages: numberFlag(argv, 'max-rendered-thread-messages', 80, 10, 500),
    maxDomNodesAfterOpen: numberFlag(argv, 'max-dom-nodes-after-open', 11000, 2000, 250000),
    maxHeapGrowthMb: numberFlag(argv, 'max-heap-growth-mb', 160, 10, 10000),
  };
}

function absPath(path: string): string {
  return resolve(ROOT, path);
}

function readText(path: string): string {
  return readFileSync(absPath(path), 'utf8');
}

function requireExists(path: string, violations: Violation[]): boolean {
  if (existsSync(absPath(path))) return true;
  violations.push({
    kind: 'missing_long_chat_ram_guard_source',
    path,
    detail: 'Required Shell long-chat RAM guard source is missing.',
  });
  return false;
}

function requireTokens(path: string, source: string, tokens: string[], kind: string, detail: string): Violation[] {
  return tokens
    .filter((token) => !source.includes(token))
    .map((token) => ({ kind, path, token, detail }));
}

function syntheticMessages(count: number): any[] {
  const baseTime = Date.UTC(2026, 3, 28, 12, 0, 0);
  const rows: any[] = [];
  for (let index = 0; index < count; index += 1) {
    rows.push({
      id: `long-chat-ram-${index}`,
      role: index % 4 === 0 ? 'user' : 'agent',
      text: `Synthetic long-chat message ${index}. `.repeat((index % 7) + 1),
      ts: new Date(baseTime + index * 1000).toISOString(),
      tools: index > 0 && index % 50 === 0
        ? [{ id: `tool-${index}`, name: 'diagnostic_probe', result: 'ok', expanded: false }]
        : [],
    });
  }
  return rows;
}

function flushStore(): Promise<void> {
  return new Promise((resolveDone) => setTimeout(resolveDone, 0));
}

async function runProjectionSmoke(args: Args): Promise<ProjectionSmoke> {
  const source = readText(args.chatStorePath);
  const windowStub: any = {};
  const sandbox: any = {
    window: windowStub,
    console,
    Promise,
    queueMicrotask,
    setTimeout,
  };
  vm.createContext(sandbox);
  vm.runInContext(source, sandbox, { filename: args.chatStorePath });
  const store = windowStub.InfringChatStore;
  if (!store || typeof store.syncMessages !== 'function' || !store.filteredMessages || !store.mapRows) {
    throw new Error('InfringChatStore did not expose syncMessages, filteredMessages, and mapRows.');
  }

  store.syncMessages([], []);
  await flushStore();
  const beforeProjected = readStoreLength(store.filteredMessages);
  const beforeMapRows = readStoreLength(store.mapRows);

  const messages = syntheticMessages(args.messageCount);
  store.syncMessages(messages, messages);
  await flushStore();
  const afterOpenProjected = readStoreLength(store.filteredMessages);
  const afterOpenMapRows = readStoreLength(store.mapRows);

  store.setThreadProjectionCenter(Math.floor(args.messageCount / 2));
  await flushStore();
  const centeredProjected = readStoreLength(store.filteredMessages);

  store.setThreadProjectionCenter(args.messageCount - 1);
  await flushStore();
  const tailProjected = readStoreLength(store.filteredMessages);

  return {
    ok: afterOpenProjected <= args.maxRenderedThreadMessages &&
      centeredProjected <= args.maxRenderedThreadMessages &&
      tailProjected <= args.maxRenderedThreadMessages,
    before_projected_messages: beforeProjected,
    before_map_rows: beforeMapRows,
    after_open_projected_messages: afterOpenProjected,
    after_open_map_rows: afterOpenMapRows,
    centered_projected_messages: centeredProjected,
    tail_projected_messages: tailProjected,
  };
}

function readStoreLength(store: any): number {
  if (!store || typeof store.get !== 'function') return -1;
  const value = store.get();
  return Array.isArray(value) ? value.length : -1;
}

function rounded(value: number): number {
  return Math.round(value * 100) / 100;
}

function buildSnapshot(label: string, messageCount: number, projectedMessages: number, mapRows: number): MemorySnapshot {
  const opened = messageCount > 0;
  const totalNodes = SYNTHETIC_BUDGET.baseDomNodes +
    (opened ? SYNTHETIC_BUDGET.shellNodesAfterOpen : 0) +
    (projectedMessages * SYNTHETIC_BUDGET.domNodesPerProjectedMessage) +
    (mapRows * SYNTHETIC_BUDGET.domNodesPerMapRow);
  const usedHeapMb = SYNTHETIC_BUDGET.baseHeapMb +
    (opened ? SYNTHETIC_BUDGET.shellHeapAfterOpenMb : 0) +
    ((messageCount * SYNTHETIC_BUDGET.heapKbPerRetainedMessage) / 1024) +
    ((projectedMessages * SYNTHETIC_BUDGET.heapKbPerProjectedMessage) / 1024) +
    ((mapRows * SYNTHETIC_BUDGET.heapKbPerMapRow) / 1024);
  return {
    label,
    message_count: messageCount,
    projected_thread_messages: projectedMessages,
    map_rows: mapRows,
    heap: {
      heap_unsupported: false,
      used_js_heap_mb: rounded(usedHeapMb),
      total_js_heap_mb: rounded(usedHeapMb + 48),
    },
    dom_counts: {
      total_nodes: totalNodes,
      scripts: 42,
      styles: 28,
      divs: Math.round(totalNodes * 0.58),
    },
    custom_element_counts: {
      'infring-chat-thread-shell': 1,
      'infring-chat-bubble-render': projectedMessages,
      'infring-message-placeholder-shell': 0,
      'infring-chat-map-shell': opened ? 1 : 0,
      'infring-chat-stream-shell': projectedMessages,
    },
  };
}

function evaluateBudgets(args: Args, smoke: ProjectionSmoke, before: MemorySnapshot, after: MemorySnapshot): Violation[] {
  const violations: Violation[] = [];
  const maxProjected = Math.max(
    smoke.after_open_projected_messages,
    smoke.centered_projected_messages,
    smoke.tail_projected_messages,
  );
  if (!smoke.ok || maxProjected > args.maxRenderedThreadMessages) {
    violations.push({
      kind: 'projected_thread_messages_unbounded',
      detail: 'Opening a large chat must not project more than the bounded Svelte thread window.',
      observed: maxProjected,
      limit: args.maxRenderedThreadMessages,
    });
  }
  const heapDelta = rounded(after.heap.used_js_heap_mb - before.heap.used_js_heap_mb);
  if (heapDelta > args.maxHeapGrowthMb) {
    violations.push({
      kind: 'estimated_heap_growth_unbounded',
      detail: 'Estimated JS heap growth for the synthetic long thread exceeds the local regression budget.',
      observed: heapDelta,
      limit: args.maxHeapGrowthMb,
    });
  }
  if (after.dom_counts.total_nodes > args.maxDomNodesAfterOpen) {
    violations.push({
      kind: 'estimated_dom_nodes_unbounded',
      detail: 'Estimated DOM nodes after opening the synthetic long thread exceed the local regression budget.',
      observed: after.dom_counts.total_nodes,
      limit: args.maxDomNodesAfterOpen,
    });
  }
  if (after.custom_element_counts['infring-chat-bubble-render'] > args.maxRenderedThreadMessages) {
    violations.push({
      kind: 'heavy_bubble_instance_count_unbounded',
      detail: 'Heavy chat bubble custom elements must remain bounded by the rendered thread window.',
      observed: after.custom_element_counts['infring-chat-bubble-render'],
      limit: args.maxRenderedThreadMessages,
    });
  }
  return violations;
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Long-Chat RAM Regression Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push(`Capture mode: ${payload.capture_mode}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- message_count: ${payload.summary.message_count}`);
  lines.push(`- max_rendered_thread_messages: ${payload.summary.max_rendered_thread_messages}`);
  lines.push(`- projected_after_open: ${payload.projection_smoke.after_open_projected_messages}`);
  lines.push(`- projected_centered: ${payload.projection_smoke.centered_projected_messages}`);
  lines.push(`- projected_tail: ${payload.projection_smoke.tail_projected_messages}`);
  lines.push(`- estimated_heap_delta_mb: ${payload.delta.used_js_heap_mb}`);
  lines.push(`- estimated_dom_delta_nodes: ${payload.delta.total_nodes}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Before');
  lines.push(`- heap_used_mb: ${payload.before.heap.used_js_heap_mb}`);
  lines.push(`- dom_nodes: ${payload.before.dom_counts.total_nodes}`);
  lines.push('');
  lines.push('## After Opening Large Thread');
  lines.push(`- heap_used_mb: ${payload.after.heap.used_js_heap_mb}`);
  lines.push(`- dom_nodes: ${payload.after.dom_counts.total_nodes}`);
  lines.push(`- chat_bubble_render_instances: ${payload.after.custom_element_counts['infring-chat-bubble-render']}`);
  lines.push(`- message_placeholder_shell_instances: ${payload.after.custom_element_counts['infring-message-placeholder-shell']}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = readArgs(argv);
  const violations: Violation[] = [];
  const paths = [
    args.chatStorePath,
    args.chatPagePartPath,
    args.threadSourcePath,
    args.placeholderSourcePath,
    args.bubbleSourcePath,
  ];
  for (const path of paths) requireExists(path, violations);

  let projectionSmoke: ProjectionSmoke = {
    ok: false,
    before_projected_messages: -1,
    before_map_rows: -1,
    after_open_projected_messages: -1,
    after_open_map_rows: -1,
    centered_projected_messages: -1,
    tail_projected_messages: -1,
  };
  let before = buildSnapshot('before_open_large_thread', 0, 0, 0);
  let after = buildSnapshot('after_open_large_thread', args.messageCount, 0, 0);

  if (!violations.length) {
    const chatPagePart = readText(args.chatPagePartPath);
    const chatStore = readText(args.chatStorePath);
    const threadSource = readText(args.threadSourcePath);
    const placeholderSource = readText(args.placeholderSourcePath);
    const bubbleSource = readText(args.bubbleSourcePath);

    violations.push(...requireTokens(args.chatPagePartPath, chatPagePart, [
      'runSlashMemprobe',
      'collectMemprobeReport',
      'performance.memory',
      "document.querySelectorAll('*')",
      'customElementTags',
      'used_js_heap_mb',
      'dom_counts',
      'delta',
      'infring-chat-bubble-render',
      'infring-message-placeholder-shell',
    ], 'missing_live_memprobe_contract_token', 'The dashboard must keep a live /memprobe path that records heap, DOM, and custom element counts.'));
    violations.push(...requireTokens(args.chatStorePath, chatStore, [
      'threadProjectionLimit = 80',
      'projectThreadMessages',
      'store.syncMessages',
      'store.setThreadProjectionCenter',
      'store.renderWindowVersion',
    ], 'missing_long_chat_store_projection_token', 'Long-chat message state must project the active Svelte thread to a bounded render slice.'));
    violations.push(...requireTokens(args.threadSourcePath, threadSource, [
      'renderWindowVersion',
      'shouldRenderMessageContent',
      'infring-chat-bubble-render',
      'infring-message-placeholder-shell',
      'messagePlaceholderLineIndices',
    ], 'missing_svelte_thread_virtualization_token', 'Svelte chat thread must preserve the heavy-bubble vs placeholder seam.'));
    violations.push(...requireTokens(args.placeholderSourcePath, placeholderSource, ['infring-message-placeholder-shell'], 'missing_placeholder_shell_token', 'Placeholder custom element must exist for off-window chat content.'));
    violations.push(...requireTokens(args.bubbleSourcePath, bubbleSource, ['infring-chat-bubble-render'], 'missing_chat_bubble_render_token', 'Heavy chat bubble renderer must remain isolated as a measurable custom element.'));

    try {
      projectionSmoke = await runProjectionSmoke(args);
      before = buildSnapshot('before_open_large_thread', 0, projectionSmoke.before_projected_messages, projectionSmoke.before_map_rows);
      after = buildSnapshot(
        'after_open_large_thread',
        args.messageCount,
        projectionSmoke.after_open_projected_messages,
        projectionSmoke.after_open_map_rows,
      );
      violations.push(...evaluateBudgets(args, projectionSmoke, before, after));
    } catch (error: any) {
      violations.push({
        kind: 'long_chat_projection_smoke_failed',
        detail: String(error && error.message || error),
      });
    }
  }

  const delta = {
    used_js_heap_mb: rounded(after.heap.used_js_heap_mb - before.heap.used_js_heap_mb),
    total_nodes: after.dom_counts.total_nodes - before.dom_counts.total_nodes,
    bubble_count: after.custom_element_counts['infring-chat-bubble-render'] - before.custom_element_counts['infring-chat-bubble-render'],
    message_count: after.message_count - before.message_count,
  };
  const payload = {
    ok: violations.length === 0,
    type: 'shell_long_chat_ram_regression_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    capture_mode: 'deterministic_store_projection_with_memprobe_contract',
    strict: args.strict,
    budgets: {
      max_rendered_thread_messages: args.maxRenderedThreadMessages,
      max_dom_nodes_after_open: args.maxDomNodesAfterOpen,
      max_heap_growth_mb: args.maxHeapGrowthMb,
      synthetic_budget: SYNTHETIC_BUDGET,
    },
    summary: {
      message_count: args.messageCount,
      max_rendered_thread_messages: args.maxRenderedThreadMessages,
      checked_sources: paths.length,
      violations: violations.length,
    },
    projection_smoke: projectionSmoke,
    before,
    after,
    delta,
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
