import { randomUUID } from 'node:crypto';
import type {
  InfringTransport,
  InfringTransportRequest,
  JsonValue,
  ReceiptPointer,
  SdkEnvelope,
} from '../types';

export const RESIDENT_IPC_TOPOLOGY = 'resident_ipc_authoritative';

export type ResidentIpcInvoker = <TData extends JsonValue = JsonValue>(
  request: InfringTransportRequest
) => Promise<SdkEnvelope<TData>> | SdkEnvelope<TData>;

export interface ResidentIpcTransportOptions {
  invoke: ResidentIpcInvoker;
  topology_mode?: string;
}

function traceId(): string {
  return `trace_${randomUUID().replace(/-/g, '')}`;
}

function emptyData<TData extends JsonValue>(): TData {
  return {} as TData;
}

function failureEnvelope<TData extends JsonValue>(
  request: InfringTransportRequest,
  code: string,
  message: string
): SdkEnvelope<TData> {
  return {
    ok: false,
    operation: request.operation,
    trace_id: traceId(),
    receipts: [],
    data: emptyData<TData>(),
    error: { code, message },
  };
}

function isReceiptArray(value: unknown): value is ReceiptPointer[] {
  return Array.isArray(value) && value.every((row) => row && typeof row === 'object');
}

function normalizeEnvelope<TData extends JsonValue>(
  request: InfringTransportRequest,
  response: SdkEnvelope<TData>
): SdkEnvelope<TData> {
  if (!response || typeof response !== 'object') {
    return failureEnvelope<TData>(
      request,
      'resident_ipc_invalid_response',
      'Resident IPC transport returned a non-envelope response.'
    );
  }
  if (!Array.isArray(response.receipts) || response.receipts.length === 0) {
    return failureEnvelope<TData>(
      request,
      'resident_ipc_missing_receipts',
      'Resident IPC transport must return receipts for every successful response.'
    );
  }
  return {
    ok: response.ok === true,
    operation: response.operation || request.operation,
    trace_id: response.trace_id || traceId(),
    receipts: isReceiptArray(response.receipts) ? response.receipts : [],
    data: response.data,
    error: response.error,
  };
}

export function createResidentIpcTransport(
  options: ResidentIpcTransportOptions
): InfringTransport {
  if (typeof options.invoke !== 'function') {
    throw new Error('sdk_resident_ipc_transport_requires_invoke');
  }
  const topologyMode = String(options.topology_mode || RESIDENT_IPC_TOPOLOGY).trim();
  return {
    async invoke<TData extends JsonValue = JsonValue>(
      request: InfringTransportRequest
    ): Promise<SdkEnvelope<TData>> {
      if (topologyMode !== RESIDENT_IPC_TOPOLOGY) {
        return failureEnvelope<TData>(
          request,
          'resident_ipc_authoritative_required',
          `Resident IPC transport requires topology mode '${RESIDENT_IPC_TOPOLOGY}'.`
        );
      }
      const response = await options.invoke<TData>(request);
      return normalizeEnvelope(request, response);
    },
  };
}
