import { asRecord, requestJson, type JsonRecord } from '$lib/api';
import { readSidebarAgents, type DashboardAgentRow } from '$lib/chat';

export type DashboardCommsNode = {
  id: string;
  name: string;
  state: string;
};

export type DashboardCommsEdge = {
  from: string;
  to: string;
  kind: string;
};

export type DashboardCommsEvent = {
  kind: string;
  ts: string;
  from_agent_id: string;
  to_agent_id: string;
  title: string;
};

export type DashboardCommsSnapshot = {
  nodes: DashboardCommsNode[];
  edges: DashboardCommsEdge[];
  events: DashboardCommsEvent[];
  agents: DashboardAgentRow[];
};

export async function readCommsSnapshot(): Promise<DashboardCommsSnapshot> {
  const [topology, events, agents] = await Promise.all([
    requestJson<JsonRecord>('GET', '/api/comms/topology'),
    requestJson<unknown>('GET', '/api/comms/events?limit=200'),
    readSidebarAgents().catch(() => []),
  ]);
  const nodes = Array.isArray(topology.nodes) ? topology.nodes : [];
  const edges = Array.isArray(topology.edges) ? topology.edges : [];
  const eventRows = Array.isArray(events) ? events : [];
  return {
    nodes: nodes.map((row) => {
      const item = asRecord(row);
      return {
        id: String(item.id || '').trim(),
        name: String(item.name || item.id || 'agent').trim(),
        state: String(item.state || 'unknown').trim(),
      };
    }),
    edges: edges.map((row) => {
      const item = asRecord(row);
      return {
        from: String(item.from || '').trim(),
        to: String(item.to || '').trim(),
        kind: String(item.kind || 'edge').trim(),
      };
    }),
    events: eventRows.map((row) => {
      const item = asRecord(row);
      return {
        kind: String(item.kind || 'event').trim(),
        ts: String(item.ts || item.created_at || '').trim(),
        from_agent_id: String(item.from_agent_id || '').trim(),
        to_agent_id: String(item.to_agent_id || '').trim(),
        title: String(item.title || item.message || item.kind || 'event').trim(),
      };
    }),
    agents,
  };
}

export async function sendCommsMessage(input: { from_agent_id: string; to_agent_id: string; message: string }): Promise<string> {
  await requestJson('POST', '/api/comms/send', input);
  return 'Message sent';
}

export async function postCommsTask(input: { title: string; description: string; assigned_to: string }): Promise<string> {
  await requestJson('POST', '/api/comms/task', input.assigned_to ? input : { title: input.title, description: input.description });
  return 'Task posted';
}
