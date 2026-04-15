import { randomUUID } from 'node:crypto';
import type {
  InfringOperation,
  InfringTransport,
  InfringTransportRequest,
  JsonValue,
  ReceiptPointer,
  SdkEnvelope,
} from './types';

export type InMemorySeed = Partial<Record<InfringOperation, JsonValue>>;

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

function toReceipts(policyRefs: string[]): ReceiptPointer[] {
  if (policyRefs.length === 0) {
    return [toReceipt()];
  }
  return policyRefs.map((policyRef) => toReceipt(policyRef));
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
    receipts: toReceipts(policyRefs),
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

export interface TestingInMemoryTransportOptions {
  unseeded_behavior?: 'error' | 'synthetic_success';
}

export function createTestingInMemoryTransport(
  seed: InMemorySeed = {},
  options: TestingInMemoryTransportOptions = {}
): InfringTransport {
  const unseededBehavior = options.unseeded_behavior ?? 'error';
  return {
    async invoke<TData extends JsonValue = JsonValue>(
      request: InfringTransportRequest
    ): Promise<SdkEnvelope<TData>> {
      const hasSeed = Object.prototype.hasOwnProperty.call(seed, request.operation);
      const seeded = hasSeed ? seed[request.operation] : undefined;
      if (!hasSeed) {
        if (unseededBehavior === 'synthetic_success') {
          return envelope(
            request.operation,
            {
              synthetic_fallback: true,
            } as TData,
            [...request.policy_refs, 'sdk.testing.synthetic_fallback']
          );
        }
        return failureEnvelope<TData>(
          request,
          'in_memory_seed_required',
          `In-memory transport missing seed for operation '${request.operation}'.`
        );
      }
      return envelope(request.operation, seeded as TData, request.policy_refs);
    },
  };
}
