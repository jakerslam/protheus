#!/usr/bin/env node
/* eslint-disable no-console */
import vm from 'node:vm';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';
import {
  SYNTHETIC_BUDGET,
  buildSnapshot,
  evaluateBudgets,
  markdown,
  rounded,
  type Args,
  type MemorySnapshot,
  type ProjectionSmoke,
  type Violation,
} from './shell_long_chat_ram_regression_guard_parts/phase_budget_model.ts';

const ROOT = process.cwd();
const DEFAULT_CHAT_STORE = 'client/runtime/systems/ui/infring_static/js/chat_store.ts';
const DEFAULT_CHAT_PAGE_PART = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/120-slash-and-agent-select.ts';
const DEFAULT_THREAD_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/chat_thread_shell_svelte_source.ts';
const DEFAULT_PLACEHOLDER_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/message_placeholder_shell_svelte_source.ts';
const DEFAULT_BUBBLE_SOURCE = 'client/runtime/systems/ui/infring_static/js/svelte/chat_bubble_svelte_source.ts';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_long_chat_ram_regression_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_LONG_CHAT_RAM_REGRESSION_GUARD_CURRENT.md';

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
    maxStorageBytes: numberFlag(argv, 'max-storage-bytes', 262144, 32768, 10_000_000),
    maxCleanupStorageBytes: numberFlag(argv, 'max-cleanup-storage-bytes', 32768, 1024, 1_000_000),
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
      text: `${index % 37 === 0 ? 'needle-match ' : ''}Synthetic long-chat message ${index}. `.repeat((index % 7) + 1),
      ts: new Date(baseTime + index * 1000).toISOString(),
      tools: index > 0 && index % 50 === 0
        ? [{
            id: `tool-${index}`,
            name: 'diagnostic_probe',
            result: 'ok',
            detail_ref: `/api/agents/chat-ui-default-agent/details/tool-result/tool-${index}`,
            expanded: false,
          }]
        : [],
      artifacts: index > 0 && index % 120 === 0
        ? [{ id: `artifact-${index}`, name: 'bounded-report', detail_ref: `/api/agents/chat-ui-default-agent/details/artifact/artifact-${index}` }]
        : [],
    });
  }
  return rows;
}

function flushStore(): Promise<void> {
  return new Promise((resolveDone) => setTimeout(resolveDone, 0));
}

function readStoreMeta(store: any): any {
  if (!store || !store.threadProjectionMeta || typeof store.threadProjectionMeta.get !== 'function') return {};
  const value = store.threadProjectionMeta.get();
  return value && typeof value === 'object' ? value : {};
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
  store.syncMessages(messages, messages);
  await flushStore();
  const scrollProjected = readStoreLength(store.filteredMessages);
  const scrollMapRows = readStoreLength(store.mapRows);
  const scrollMeta = readStoreMeta(store);

  const searchRows = messages.filter((message) => String(message.text || '').includes('needle-match'));
  store.syncMessages(messages, searchRows);
  await flushStore();
  const searchProjected = readStoreLength(store.filteredMessages);
  const searchMapRows = readStoreLength(store.mapRows);

  const toolMessage = messages.find((message) => Array.isArray(message.tools) && message.tools.length);
  const toolDetailBytes = toolMessage ? Buffer.byteLength(JSON.stringify(toolMessage.tools[0] || {}), 'utf8') : 0;
  store.syncMessages(messages, messages);
  await flushStore();
  const toolDetailProjected = readStoreLength(store.filteredMessages);
  const toolDetailMapRows = readStoreLength(store.mapRows);

  const sessionSwitchMessageCount = Math.max(100, Math.floor(args.messageCount * 0.5));
  const switchedMessages = syntheticMessages(sessionSwitchMessageCount).map((message, index) => ({
    ...message,
    id: `long-chat-ram-session-b-${index}`,
  }));
  store.setThreadProjectionCenter(-1);
  store.syncMessages(switchedMessages, switchedMessages);
  await flushStore();
  const sessionSwitchProjected = readStoreLength(store.filteredMessages);
  const sessionSwitchMapRows = readStoreLength(store.mapRows);

  store.syncMessages([], []);
  await flushStore();
  const cleanupProjected = readStoreLength(store.filteredMessages);
  const cleanupMapRows = readStoreLength(store.mapRows);

  return {
    ok: afterOpenProjected <= args.maxRenderedThreadMessages &&
      scrollProjected <= args.maxRenderedThreadMessages &&
      searchProjected <= args.maxRenderedThreadMessages &&
      toolDetailProjected <= args.maxRenderedThreadMessages &&
      sessionSwitchProjected <= args.maxRenderedThreadMessages &&
      cleanupProjected === 0 &&
      cleanupMapRows === 0,
    before_projected_messages: beforeProjected,
    before_map_rows: beforeMapRows,
    after_open_projected_messages: afterOpenProjected,
    after_open_map_rows: afterOpenMapRows,
    scroll_projected_messages: scrollProjected,
    scroll_map_rows: scrollMapRows,
    scroll_window_start_index: Number(scrollMeta.windowStartIndex ?? -1),
    scroll_window_end_index: Number(scrollMeta.windowEndIndex ?? -1),
    search_projected_messages: searchProjected,
    search_map_rows: searchMapRows,
    search_total_matches: searchRows.length,
    tool_detail_projected_messages: toolDetailProjected,
    tool_detail_map_rows: toolDetailMapRows,
    tool_detail_bytes: toolDetailBytes,
    session_switch_message_count: sessionSwitchMessageCount,
    session_switch_projected_messages: sessionSwitchProjected,
    session_switch_map_rows: sessionSwitchMapRows,
    cleanup_projected_messages: cleanupProjected,
    cleanup_map_rows: cleanupMapRows,
  };
}

function readStoreLength(store: any): number {
  if (!store || typeof store.get !== 'function') return -1;
  const value = store.get();
  return Array.isArray(value) ? value.length : -1;
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
    scroll_projected_messages: -1,
    scroll_map_rows: -1,
    scroll_window_start_index: -1,
    scroll_window_end_index: -1,
    search_projected_messages: -1,
    search_map_rows: -1,
    search_total_matches: -1,
    tool_detail_projected_messages: -1,
    tool_detail_map_rows: -1,
    tool_detail_bytes: -1,
    session_switch_message_count: -1,
    session_switch_projected_messages: -1,
    session_switch_map_rows: -1,
    cleanup_projected_messages: -1,
    cleanup_map_rows: -1,
  };
  let before = buildSnapshot('before_open_large_thread', 0, 0, 0);
  let after = buildSnapshot('after_open_large_thread', args.messageCount, 0, 0);
  let phaseSnapshots: MemorySnapshot[] = [before, after];

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
      'storage_bytes',
      'total_storage_bytes',
      'delta',
      'infring-chat-bubble-render',
      'infring-message-placeholder-shell',
    ], 'missing_live_memprobe_contract_token', 'The dashboard must keep a live /memprobe path that records heap, DOM, storage, and custom element counts.'));
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
      const afterScroll = buildSnapshot(
        'after_scroll',
        args.messageCount,
        projectionSmoke.scroll_projected_messages,
        projectionSmoke.scroll_map_rows,
      );
      const afterSearch = buildSnapshot(
        'after_search',
        args.messageCount,
        projectionSmoke.search_projected_messages,
        projectionSmoke.search_map_rows,
        { extraDomNodes: SYNTHETIC_BUDGET.searchDomNodes, extraHeapMb: SYNTHETIC_BUDGET.searchHeapMb },
      );
      const afterToolDetail = buildSnapshot(
        'after_tool_detail_expansion',
        args.messageCount,
        projectionSmoke.tool_detail_projected_messages,
        projectionSmoke.tool_detail_map_rows,
        {
          detailExpanded: true,
          extraDomNodes: SYNTHETIC_BUDGET.toolDetailDomNodes,
          extraHeapMb: SYNTHETIC_BUDGET.toolDetailHeapMb,
          extraStorageBytes: Math.min(projectionSmoke.tool_detail_bytes, SYNTHETIC_BUDGET.toolDetailStorageBytes),
        },
      );
      const afterSessionSwitch = buildSnapshot(
        'after_session_switch',
        projectionSmoke.session_switch_message_count,
        projectionSmoke.session_switch_projected_messages,
        projectionSmoke.session_switch_map_rows,
        { extraHeapMb: SYNTHETIC_BUDGET.sessionSwitchHeapMb },
      );
      const afterCleanup = buildSnapshot(
        'after_cleanup',
        0,
        projectionSmoke.cleanup_projected_messages,
        projectionSmoke.cleanup_map_rows,
      );
      phaseSnapshots = [before, after, afterScroll, afterSearch, afterToolDetail, afterSessionSwitch, afterCleanup];
      violations.push(...evaluateBudgets(args, projectionSmoke, phaseSnapshots));
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
    total_storage_bytes: after.storage_bytes.total_storage_bytes - before.storage_bytes.total_storage_bytes,
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
      max_storage_bytes: args.maxStorageBytes,
      max_cleanup_storage_bytes: args.maxCleanupStorageBytes,
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
    phase_snapshots: phaseSnapshots,
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
