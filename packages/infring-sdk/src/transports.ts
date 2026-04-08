import { randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import type {
  AttachPoliciesData,
  InfringOperation,
  InfringTransport,
  InfringTransportRequest,
  InspectReceiptsData,
  JsonObject,
  JsonValue,
  QueryMemoryData,
  ReceiptPointer,
  ReviewEvidenceData,
  RunAssimilationData,
  SdkEnvelope,
  SubmitTaskData,
} from './types';

function nowIso(): string {
  return new Date().toISOString();
}

function toReceipt(policyRef?: string): ReceiptPointer {
  return {
    receipt_id: `receipt_${randomUUID().replace(/-/g, '')}`,
    issued_at: nowIso(),
    policy_ref: policyRef,
  };
}

function envelope<TData extends JsonValue>(
  operation: InfringOperation,
  data: TData,
  policyRefs: string[]
): SdkEnvelope<TData> {
  return {
    ok: true,
    operation,
    trace_id: `trace_${randomUUID().replace(/-/g, '')}`,
    receipts: [toReceipt(policyRefs[0])],
    data,
  };
}

function parseJsonLine(stdout: string): JsonObject | null {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      const parsed = JSON.parse(lines[i]);
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        return parsed as JsonObject;
      }
    } catch {}
  }
  return null;
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((row) => String(row || '').trim())
    .filter((row) => row.length > 0);
}

function defaultSubmitTaskData(request: InfringTransportRequest): SubmitTaskData {
  const explicit = String((request.payload.task_id as string) || '').trim();
  return {
    task_id: explicit || `task_${randomUUID().slice(0, 12)}`,
    accepted: true,
    status: 'queued',
  };
}

function defaultInspectReceiptsData(): InspectReceiptsData {
  return {
    receipts: [],
  };
}

function defaultQueryMemoryData(): QueryMemoryData {
  return {
    records: [],
  };
}

function defaultReviewEvidenceData(): ReviewEvidenceData {
  return {
    evidence: [],
  };
}

function defaultRunAssimilationData(request: InfringTransportRequest): RunAssimilationData {
  const target = String((request.payload.target as string) || '').trim() || 'unknown';
  return {
    assimilation_id: `assim_${randomUUID().slice(0, 12)}`,
    admitted: true,
    status: target.length > 0 ? 'planned' : 'rejected',
  };
}

function defaultAttachPoliciesData(request: InfringTransportRequest): AttachPoliciesData {
  const payloadPolicies = Array.isArray(request.payload.policies)
    ? (request.payload.policies as Array<{ policy_ref?: string }>)
    : [];
  const refs = payloadPolicies
    .map((row) => String(row && row.policy_ref ? row.policy_ref : '').trim())
    .filter((row) => row.length > 0);
  return {
    applied_policy_refs: refs,
  };
}

function defaultDataForOperation(request: InfringTransportRequest): JsonValue {
  switch (request.operation) {
    case 'submit_task':
      return defaultSubmitTaskData(request);
    case 'inspect_receipts':
      return defaultInspectReceiptsData();
    case 'query_memory':
      return defaultQueryMemoryData();
    case 'review_evidence':
      return defaultReviewEvidenceData();
    case 'run_assimilation':
      return defaultRunAssimilationData(request);
    case 'attach_policies':
      return defaultAttachPoliciesData(request);
    default:
      return {};
  }
}

export interface CliTransportOptions {
  command: string;
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  timeout_ms?: number;
  args_for_operation: (request: InfringTransportRequest) => string[];
}

export function createCliTransport(options: CliTransportOptions): InfringTransport {
  const command = String(options.command || '').trim();
  if (!command) {
    throw new Error('sdk_cli_transport_requires_command');
  }
  if (typeof options.args_for_operation !== 'function') {
    throw new Error('sdk_cli_transport_requires_args_for_operation');
  }
  return {
    async invoke<TData extends JsonValue = JsonValue>(
      request: InfringTransportRequest
    ): Promise<SdkEnvelope<TData>> {
      const args = options.args_for_operation(request);
      const proc = spawnSync(command, args, {
        cwd: options.cwd,
        env: { ...process.env, ...(options.env || {}) },
        encoding: 'utf8',
        timeout: Math.max(1000, Number(options.timeout_ms || 120000)),
      });
      const parsed = parseJsonLine(String(proc.stdout || '')) || {};
      const receipts = asStringArray(parsed.receipts).map((receiptId) => ({
        receipt_id: receiptId,
        issued_at: nowIso(),
      }));
      const data = (parsed.data as TData) || (defaultDataForOperation(request) as TData);
      if (Number(proc.status || 0) !== 0) {
        return {
          ok: false,
          operation: request.operation,
          trace_id: String(parsed.trace_id || `trace_${randomUUID().replace(/-/g, '')}`),
          receipts,
          data,
        };
      }
      return {
        ok: true,
        operation: request.operation,
        trace_id: String(parsed.trace_id || `trace_${randomUUID().replace(/-/g, '')}`),
        receipts: receipts.length > 0 ? receipts : [toReceipt(request.policy_refs[0])],
        data,
      };
    },
  };
}

export type InMemorySeed = Partial<Record<InfringOperation, JsonValue>>;

export function createInMemoryTransport(seed: InMemorySeed = {}): InfringTransport {
  return {
    async invoke<TData extends JsonValue = JsonValue>(
      request: InfringTransportRequest
    ): Promise<SdkEnvelope<TData>> {
      const seeded = seed[request.operation];
      const data = (seeded as TData) || (defaultDataForOperation(request) as TData);
      return envelope(request.operation, data, request.policy_refs);
    },
  };
}
