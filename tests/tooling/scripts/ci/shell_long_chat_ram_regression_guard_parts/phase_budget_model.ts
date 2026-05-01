/* eslint-disable no-console */
export type Args = {
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
  maxStorageBytes: number;
  maxCleanupStorageBytes: number;
};

export type Violation = {
  kind: string;
  path?: string;
  token?: string;
  detail: string;
  observed?: number;
  limit?: number;
};

export type MemorySnapshot = {
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
  storage_bytes: {
    local_storage_bytes: number;
    session_storage_bytes: number;
    total_storage_bytes: number;
  };
};

export type ProjectionSmoke = {
  ok: boolean;
  before_projected_messages: number;
  before_map_rows: number;
  after_open_projected_messages: number;
  after_open_map_rows: number;
  scroll_projected_messages: number;
  scroll_map_rows: number;
  scroll_window_start_index: number;
  scroll_window_end_index: number;
  search_projected_messages: number;
  search_map_rows: number;
  search_total_matches: number;
  tool_detail_projected_messages: number;
  tool_detail_map_rows: number;
  tool_detail_bytes: number;
  session_switch_message_count: number;
  session_switch_projected_messages: number;
  session_switch_map_rows: number;
  cleanup_projected_messages: number;
  cleanup_map_rows: number;
};

export const SYNTHETIC_BUDGET = {
  baseDomNodes: 1600,
  shellNodesAfterOpen: 220,
  domNodesPerProjectedMessage: 38,
  domNodesPerMapRow: 5,
  baseHeapMb: 96,
  shellHeapAfterOpenMb: 8,
  heapKbPerRetainedMessage: 16,
  heapKbPerProjectedMessage: 560,
  heapKbPerMapRow: 3,
  storageBaseBytes: 5_120,
  storageBytesPerRetainedMessage: 8,
  storageBytesPerProjectedMessage: 256,
  storageBytesPerMapRow: 96,
  searchDomNodes: 60,
  searchHeapMb: 2,
  toolDetailDomNodes: 140,
  toolDetailHeapMb: 3,
  toolDetailStorageBytes: 8_192,
  sessionSwitchHeapMb: 4,
};

function numberFlag(argv: string[], name: string, fallback: number, min: number, max: number): number {
  const raw = readFlag(argv, name);
  const parsed = Number(raw == null || raw === '' ? fallback : raw);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(min, Math.min(max, Math.round(parsed)));
}
export function rounded(value: number): number {
  return Math.round(value * 100) / 100;
}

export function buildSnapshot(
  label: string,
  messageCount: number,
  projectedMessages: number,
  mapRows: number,
  options: {
    extraDomNodes?: number;
    extraHeapMb?: number;
    extraStorageBytes?: number;
    detailExpanded?: boolean;
  } = {},
): MemorySnapshot {
  const opened = messageCount > 0;
  const totalNodes = SYNTHETIC_BUDGET.baseDomNodes +
    (opened ? SYNTHETIC_BUDGET.shellNodesAfterOpen : 0) +
    (projectedMessages * SYNTHETIC_BUDGET.domNodesPerProjectedMessage) +
    (mapRows * SYNTHETIC_BUDGET.domNodesPerMapRow) +
    Math.max(0, Math.round(options.extraDomNodes || 0));
  const usedHeapMb = SYNTHETIC_BUDGET.baseHeapMb +
    (opened ? SYNTHETIC_BUDGET.shellHeapAfterOpenMb : 0) +
    ((messageCount * SYNTHETIC_BUDGET.heapKbPerRetainedMessage) / 1024) +
    ((projectedMessages * SYNTHETIC_BUDGET.heapKbPerProjectedMessage) / 1024) +
    ((mapRows * SYNTHETIC_BUDGET.heapKbPerMapRow) / 1024) +
    Math.max(0, Number(options.extraHeapMb || 0));
  const storageBytes = SYNTHETIC_BUDGET.storageBaseBytes +
    (messageCount * SYNTHETIC_BUDGET.storageBytesPerRetainedMessage) +
    (projectedMessages * SYNTHETIC_BUDGET.storageBytesPerProjectedMessage) +
    (mapRows * SYNTHETIC_BUDGET.storageBytesPerMapRow) +
    Math.max(0, Math.round(options.extraStorageBytes || 0));
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
      'infring-tool-detail-shell': options.detailExpanded ? 1 : 0,
    },
    storage_bytes: {
      local_storage_bytes: Math.round(storageBytes * 0.82),
      session_storage_bytes: Math.round(storageBytes * 0.18),
      total_storage_bytes: storageBytes,
    },
  };
}

export function evaluateBudgets(args: Args, smoke: ProjectionSmoke, snapshots: MemorySnapshot[]): Violation[] {
  const violations: Violation[] = [];
  const before = snapshots[0];
  const maxProjected = Math.max(
    smoke.after_open_projected_messages,
    smoke.scroll_projected_messages,
    smoke.search_projected_messages,
    smoke.tool_detail_projected_messages,
    smoke.session_switch_projected_messages,
  );
  if (!smoke.ok || maxProjected > args.maxRenderedThreadMessages) {
    violations.push({
      kind: 'projected_thread_messages_unbounded',
      detail: 'Opening a large chat must not project more than the bounded Svelte thread window.',
      observed: maxProjected,
      limit: args.maxRenderedThreadMessages,
    });
  }
  if (smoke.scroll_window_start_index <= 0 || smoke.scroll_window_end_index <= smoke.scroll_window_start_index) {
    violations.push({
      kind: 'long_chat_scroll_window_not_exercised',
      detail: 'Synthetic scroll phase must move the projected window instead of only reopening the tail.',
      observed: smoke.scroll_window_start_index,
      limit: 1,
    });
  }
  if (smoke.search_total_matches <= 0 || smoke.search_projected_messages > smoke.search_total_matches) {
    violations.push({
      kind: 'long_chat_search_phase_invalid',
      detail: 'Synthetic search phase must produce bounded matched rows without inflating projected rows beyond matches.',
      observed: smoke.search_projected_messages,
      limit: Math.max(0, smoke.search_total_matches),
    });
  }
  if (smoke.tool_detail_bytes <= 0 || smoke.tool_detail_bytes > SYNTHETIC_BUDGET.toolDetailStorageBytes) {
    violations.push({
      kind: 'tool_detail_expansion_unbounded_or_missing',
      detail: 'Synthetic detail expansion must fetch a bounded tool-detail payload instead of embedding raw detail in the row.',
      observed: smoke.tool_detail_bytes,
      limit: SYNTHETIC_BUDGET.toolDetailStorageBytes,
    });
  }
  if (smoke.cleanup_projected_messages !== 0 || smoke.cleanup_map_rows !== 0) {
    violations.push({
      kind: 'long_chat_cleanup_retains_rows',
      detail: 'Cleanup must release projected message rows and map rows after session clear/unload.',
      observed: smoke.cleanup_projected_messages + smoke.cleanup_map_rows,
      limit: 0,
    });
  }
  for (const snapshot of snapshots.slice(1)) {
    const heapDelta = rounded(snapshot.heap.used_js_heap_mb - before.heap.used_js_heap_mb);
    if (heapDelta > args.maxHeapGrowthMb) {
      violations.push({
        kind: 'estimated_heap_growth_unbounded',
        detail: `${snapshot.label} estimated JS heap growth exceeds the local regression budget.`,
        observed: heapDelta,
        limit: args.maxHeapGrowthMb,
      });
    }
    if (snapshot.dom_counts.total_nodes > args.maxDomNodesAfterOpen) {
      violations.push({
        kind: 'estimated_dom_nodes_unbounded',
        detail: `${snapshot.label} estimated DOM nodes exceed the local regression budget.`,
        observed: snapshot.dom_counts.total_nodes,
        limit: args.maxDomNodesAfterOpen,
      });
    }
    if (snapshot.storage_bytes.total_storage_bytes > args.maxStorageBytes && snapshot.label !== 'after_cleanup') {
      violations.push({
        kind: 'estimated_storage_bytes_unbounded',
        detail: `${snapshot.label} estimated storage bytes exceed the local regression budget.`,
        observed: snapshot.storage_bytes.total_storage_bytes,
        limit: args.maxStorageBytes,
      });
    }
    if (snapshot.label === 'after_cleanup' && snapshot.storage_bytes.total_storage_bytes > args.maxCleanupStorageBytes) {
      violations.push({
        kind: 'cleanup_storage_bytes_unbounded',
        detail: 'Cleanup must leave only bounded baseline Shell storage.',
        observed: snapshot.storage_bytes.total_storage_bytes,
        limit: args.maxCleanupStorageBytes,
      });
    }
    if (snapshot.custom_element_counts['infring-chat-bubble-render'] > args.maxRenderedThreadMessages) {
      violations.push({
        kind: 'heavy_bubble_instance_count_unbounded',
        detail: `${snapshot.label} heavy chat bubble custom elements must remain bounded by the rendered thread window.`,
        observed: snapshot.custom_element_counts['infring-chat-bubble-render'],
        limit: args.maxRenderedThreadMessages,
      });
    }
  }
  return violations;
}

export function markdown(payload: any): string {
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
  lines.push(`- projected_after_scroll: ${payload.projection_smoke.scroll_projected_messages}`);
  lines.push(`- projected_after_search: ${payload.projection_smoke.search_projected_messages}`);
  lines.push(`- projected_after_tool_detail: ${payload.projection_smoke.tool_detail_projected_messages}`);
  lines.push(`- projected_after_session_switch: ${payload.projection_smoke.session_switch_projected_messages}`);
  lines.push(`- projected_after_cleanup: ${payload.projection_smoke.cleanup_projected_messages}`);
  lines.push(`- estimated_heap_delta_mb: ${payload.delta.used_js_heap_mb}`);
  lines.push(`- estimated_dom_delta_nodes: ${payload.delta.total_nodes}`);
  lines.push(`- estimated_storage_delta_bytes: ${payload.delta.total_storage_bytes}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Before');
  lines.push(`- heap_used_mb: ${payload.before.heap.used_js_heap_mb}`);
  lines.push(`- dom_nodes: ${payload.before.dom_counts.total_nodes}`);
  lines.push('');
  lines.push('## After Opening Large Thread');
  lines.push(`- heap_used_mb: ${payload.after.heap.used_js_heap_mb}`);
  lines.push(`- dom_nodes: ${payload.after.dom_counts.total_nodes}`);
  lines.push(`- storage_bytes: ${payload.after.storage_bytes.total_storage_bytes}`);
  lines.push(`- chat_bubble_render_instances: ${payload.after.custom_element_counts['infring-chat-bubble-render']}`);
  lines.push(`- message_placeholder_shell_instances: ${payload.after.custom_element_counts['infring-message-placeholder-shell']}`);
  lines.push('');
  lines.push('## Dynamic Phases');
  for (const snapshot of payload.phase_snapshots || []) {
    lines.push(`- ${snapshot.label}: heap=${snapshot.heap.used_js_heap_mb}MB, dom=${snapshot.dom_counts.total_nodes}, storage=${snapshot.storage_bytes.total_storage_bytes}, bubbles=${snapshot.custom_element_counts['infring-chat-bubble-render']}`);
  }
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}
