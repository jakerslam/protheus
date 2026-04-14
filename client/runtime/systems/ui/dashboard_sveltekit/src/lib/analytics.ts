import { asRecord, requestJson, type JsonRecord } from '$lib/api';

export type AnalyticsSummary = {
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost_usd: number;
  call_count: number;
  total_tool_calls: number;
};

export type AnalyticsModelRow = {
  model: string;
  call_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost_usd: number;
};

export type AnalyticsAgentRow = {
  agent_id: string;
  agent_name: string;
  total_tokens: number;
  tool_calls: number;
  cost_usd: number;
};

export type AnalyticsDailyCostRow = {
  date: string;
  cost_usd: number;
};

export type AnalyticsSnapshot = {
  summary: AnalyticsSummary;
  byModel: AnalyticsModelRow[];
  byAgent: AnalyticsAgentRow[];
  dailyCosts: AnalyticsDailyCostRow[];
  todayCost: number;
  firstEventDate: string;
};

export async function readAnalyticsSnapshot(): Promise<AnalyticsSnapshot> {
  const [summary, byModel, byAgent, daily] = await Promise.all([
    requestJson<JsonRecord>('GET', '/api/usage/summary'),
    requestJson<JsonRecord>('GET', '/api/usage/by-model'),
    requestJson<JsonRecord>('GET', '/api/usage'),
    requestJson<JsonRecord>('GET', '/api/usage/daily'),
  ]);
  const modelRows = Array.isArray(byModel.models) ? byModel.models : [];
  const agentRows = Array.isArray(byAgent.agents) ? byAgent.agents : [];
  const dayRows = Array.isArray(daily.days) ? daily.days : [];
  return {
    summary: {
      total_input_tokens: Number(summary.total_input_tokens || 0) || 0,
      total_output_tokens: Number(summary.total_output_tokens || 0) || 0,
      total_cost_usd: Number(summary.total_cost_usd || 0) || 0,
      call_count: Number(summary.call_count || 0) || 0,
      total_tool_calls: Number(summary.total_tool_calls || 0) || 0,
    },
    byModel: modelRows.map((row) => {
      const item = asRecord(row);
      return {
        model: String(item.model || 'unknown').trim(),
        call_count: Number(item.call_count || 0) || 0,
        total_input_tokens: Number(item.total_input_tokens || 0) || 0,
        total_output_tokens: Number(item.total_output_tokens || 0) || 0,
        total_cost_usd: Number(item.total_cost_usd || 0) || 0,
      };
    }),
    byAgent: agentRows.map((row) => {
      const item = asRecord(row);
      return {
        agent_id: String(item.agent_id || '').trim(),
        agent_name: String(item.agent_name || item.agent_id || 'agent').trim(),
        total_tokens: Number(item.total_tokens || item.total_input_tokens || 0) || 0,
        tool_calls: Number(item.tool_calls || 0) || 0,
        cost_usd: Number(item.cost_usd || item.total_cost_usd || 0) || 0,
      };
    }),
    dailyCosts: dayRows.map((row) => {
      const item = asRecord(row);
      return {
        date: String(item.date || '').trim(),
        cost_usd: Number(item.cost_usd || item.total_cost_usd || 0) || 0,
      };
    }),
    todayCost: Number(daily.today_cost_usd || 0) || 0,
    firstEventDate: String(daily.first_event_date || '').trim(),
  };
}
