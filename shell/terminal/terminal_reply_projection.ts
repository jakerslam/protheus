#!/usr/bin/env node
import type { ShellSocketGatewayClient } from '../socket/client/shell_socket_gateway_client.ts';

type MessageRow = { id?: string; role?: string; text?: string; content_preview?: string };
export type TerminalAgentSelection = { agentId: string; label: string; count: number; source: 'active' | 'history' | 'first' | 'none'; error?: string };

export type TerminalReplyProjection = {
  sessionId: string;
  messageCount: number;
  rows: MessageRow[];
};

export type TerminalSubmitResult = { accepted: boolean; label: string };

function clean(value: unknown, max = 1200): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function rowsFromWindow(payload: Record<string, unknown>): MessageRow[] {
  const messageWindow = payload.message_window && typeof payload.message_window === 'object'
    ? payload.message_window as Record<string, unknown>
    : {};
  return Array.isArray(messageWindow.rows) ? messageWindow.rows as MessageRow[] : [];
}

function labelMapValue(labels: unknown, key: string): string {
  if (!labels || typeof labels !== 'object') return '';
  return clean((labels as Record<string, unknown>)[key], 120);
}

export async function selectTerminalDefaultAgent(client: ShellSocketGatewayClient): Promise<TerminalAgentSelection> {
  try {
    const roster = (await client.listAgents<Record<string, unknown>>({ limit: 20 })) || {};
    const ids = Array.isArray(roster.agent_ids) ? roster.agent_ids.map((id) => clean(id, 120)).filter(Boolean) : [];
    const active = clean(roster.active_agent_id, 120);
    if (active) return { agentId: active, label: labelMapValue(roster.labels, active) || active, count: ids.length, source: 'active' };
    let best = { agentId: ids[0] || '', count: -1 };
    for (const id of ids.slice(0, 10)) {
      try {
        const projection = await terminalSessionProjection(client, id);
        if (projection.messageCount > best.count) best = { agentId: id, count: projection.messageCount };
      } catch (_) {
        // Keep selection fail-soft; a stale agent row should not block the terminal.
      }
    }
    const agentId = best.agentId;
    if (!agentId) return { agentId: '', label: 'No agent selected', count: ids.length, source: 'none' };
    return { agentId, label: labelMapValue(roster.labels, agentId) || agentId, count: ids.length, source: best.count > 0 ? 'history' : 'first' };
  } catch (error) {
    return { agentId: '', label: 'No agent selected', count: 0, source: 'none', error: clean(error instanceof Error ? error.message : error, 240) };
  }
}

export async function terminalSessionProjection(
  client: ShellSocketGatewayClient,
  agentId: string,
): Promise<TerminalReplyProjection> {
  const sessions = await client.listSessions<Record<string, unknown>>(agentId, { limit: 1 });
  const sessionId = clean(sessions.active_session_id || `${agentId}::default`, 180);
  const counts = sessions.message_counts && typeof sessions.message_counts === 'object'
    ? sessions.message_counts as Record<string, unknown>
    : {};
  return { sessionId, messageCount: Number(counts[sessionId] || 0), rows: [] };
}

export async function submitTerminalUserInput(
  client: ShellSocketGatewayClient,
  agentId: string,
  message: string,
  attempts = 3,
): Promise<TerminalSubmitResult> {
  if (!agentId) return { accepted: false, label: 'Select an agent first with /agents or /use <agent_id>.' };
  let lastError = '';
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      const ack = await client.submitInput<Record<string, unknown>>({ agent_id: agentId, message, source: 'terminal_shell', medium: 'terminal' });
      const accepted = ack.accepted === true && ack.rejected !== true;
      const reason = clean(ack.reason_code || (accepted ? 'accepted' : 'rejected'), 160);
      const receipt = clean(ack.receipt_ref || '', 180);
      const followUp = clean(ack.follow_up_ref || '', 180);
      return { accepted, label: accepted ? `Accepted by Gateway.${receipt ? ` receipt: ${receipt}` : ''}${followUp ? ` follow-up: ${followUp}` : ''}` : `Rejected by Gateway: ${reason}${receipt ? ` receipt: ${receipt}` : ''}` };
    } catch (error) {
      lastError = clean(error instanceof Error ? error.message : error, 300);
      await sleep(350);
    }
  }
  return { accepted: false, label: lastError || 'Gateway submit failed.' };
}

export async function pollTerminalReplyProjection(
  client: ShellSocketGatewayClient,
  agentId: string,
  beforeCount: number,
  options: { attempts?: number; intervalMs?: number; limit?: number } = {},
): Promise<TerminalReplyProjection> {
  const attempts = Math.max(1, Math.min(30, Number(options.attempts || 12)));
  const intervalMs = Math.max(100, Math.min(3000, Number(options.intervalMs || 700)));
  const limit = Math.max(1, Math.min(80, Number(options.limit || 20)));
  let projection = await terminalSessionProjection(client, agentId);
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    const payload = await client.getMessageWindow<Record<string, unknown>>(projection.sessionId, { limit });
    const total = Number(payload.total_count || payload.message_count || projection.messageCount || 0);
    const rows = rowsFromWindow(payload);
    const newCount = Math.max(0, total - beforeCount);
    const newRows = newCount > 0 ? rows.slice(-Math.min(newCount, rows.length)) : [];
    const assistantRows = newRows.filter((row) => clean(row.role, 40) === 'assistant' && clean(row.text || row.content_preview, 8000));
    projection = { sessionId: clean(payload.session_id || projection.sessionId, 180), messageCount: total, rows: assistantRows };
    if (assistantRows.length > 0) return projection;
    await sleep(intervalMs);
  }
  return projection;
}
