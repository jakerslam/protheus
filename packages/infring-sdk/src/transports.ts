import { randomUUID } from 'node:crypto';
import type {
  InfringOperation,
  InfringTransport,
  InfringTransportRequest,
  JsonValue,
  ReceiptPointer,
  SdkEnvelope,
} from './types';
// Production SDK transport surface: resident_ipc_authoritative only.
export {
  RESIDENT_IPC_TOPOLOGY,
  createResidentIpcTransport,
} from './transports/resident_ipc';
export const PRODUCTION_TRANSPORT_SURFACE = 'resident_ipc_only';
export type {
  ResidentIpcInvoker,
  ResidentIpcTransportOptions,
} from './transports/resident_ipc';

function nowIso(): string {
  return new Date().toISOString();
}

function traceId(): string {
  return `trace_${randomUUID().replace(/-/g, '')}`;
}

function emptyData<TData extends JsonValue>(): TData {
  return {} as TData;
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
    trace_id: traceId(),
    receipts: [toReceipt(policyRefs[0])],
    data,
  };
}

function failureEnvelope<TData extends JsonValue>(
  request: InfringTransportRequest,
  code: string,
  message: string,
  data: TData = emptyData<TData>()
): SdkEnvelope<TData> {
  return {
    ok: false,
    operation: request.operation,
    trace_id: traceId(),
    receipts: [],
    data,
    error: { code, message },
  };
}

export interface InMemoryTransportOptions {
  allow_unseeded_fallback?: boolean;
}

export type InMemorySeed = Partial<Record<InfringOperation, JsonValue>>;

export function createInMemoryTransport(
  seed: InMemorySeed = {},
  options: InMemoryTransportOptions = {}
): InfringTransport {
  const allowUnseededFallback = options.allow_unseeded_fallback === true;
  return {
    async invoke<TData extends JsonValue = JsonValue>(
      request: InfringTransportRequest
    ): Promise<SdkEnvelope<TData>> {
      const hasSeed = Object.prototype.hasOwnProperty.call(seed, request.operation);
      const seeded = hasSeed ? seed[request.operation] : undefined;
      if (!hasSeed) {
        return failureEnvelope<TData>(
          request,
          'in_memory_seed_required',
          allowUnseededFallback
            ? `Synthetic in-memory fallback disabled; provide a seed for '${request.operation}'.`
            : `In-memory transport missing seed for operation '${request.operation}'.`
        );
      }
      return envelope(request.operation, seeded as TData, request.policy_refs);
    },
  };
}
