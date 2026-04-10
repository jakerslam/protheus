import { randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import type {
  InfringOperation,
  InfringTransport,
  InfringTransportRequest,
  JsonObject,
  JsonValue,
  ReceiptPointer,
  SdkEnvelope,
} from './types';

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

function releaseChannel(env: NodeJS.ProcessEnv = process.env): string {
  const raw =
    String(env.INFRING_RELEASE_CHANNEL || env.PROTHEUS_RELEASE_CHANNEL || '').trim().toLowerCase();
  return raw || 'stable';
}

function isProductionReleaseChannel(channel: string): boolean {
  const normalized = String(channel || '').trim().toLowerCase();
  return (
    normalized === 'stable' ||
    normalized === 'production' ||
    normalized === 'prod' ||
    normalized === 'ga' ||
    normalized === 'release'
  );
}

export interface CliTransportOptions {
  command: string;
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  timeout_ms?: number;
  /**
   * Process transport is disabled by default.
   * Set this to true (and/or INFRING_SDK_ALLOW_PROCESS_TRANSPORT=1) only for emergency fallback.
   */
  allow_process_transport?: boolean;
  /**
   * Deprecated. Production transports are fail-closed by default.
   * Synthetic fallback requires both this option and INFRING_SDK_ALLOW_SYNTHETIC_FALLBACK=1.
   */
  allow_synthetic_fallback?: boolean;
  args_for_operation: (request: InfringTransportRequest) => string[];
}

export interface InMemoryTransportOptions {
  /**
   * Deprecated. Unseeded fallback is disabled to keep transport behavior deterministic.
   */
  allow_unseeded_fallback?: boolean;
}

type SpawnResult = {
  status: number;
  stdout: string;
  stderr: string;
  error: Error | null;
};

function runSpawn(
  command: string,
  args: string[],
  cwd: string | undefined,
  env: NodeJS.ProcessEnv,
  timeoutMs: number
): Promise<SpawnResult> {
  return new Promise((resolve) => {
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    let settled = false;
    const finish = (result: SpawnResult) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(result);
    };
    const timer = setTimeout(() => {
      try {
        child.kill('SIGKILL');
      } catch {}
      finish({
        status: 1,
        stdout,
        stderr: `${stderr}\ntransport_timeout`,
        error: new Error(`transport_timeout:${timeoutMs}`),
      });
    }, Math.max(1000, timeoutMs));
    child.stdout?.on('data', (chunk) => {
      stdout += String(chunk || '');
    });
    child.stderr?.on('data', (chunk) => {
      stderr += String(chunk || '');
    });
    child.on('error', (error) => {
      finish({
        status: 1,
        stdout,
        stderr: `${stderr}\n${String(error && error.message ? error.message : error)}`,
        error: error instanceof Error ? error : new Error(String(error)),
      });
    });
    child.on('close', (code) => {
      finish({
        status: Number.isFinite(Number(code)) ? Number(code) : 1,
        stdout,
        stderr,
        error: null,
      });
    });
  });
}

function failureEnvelope<TData extends JsonValue>(
  request: InfringTransportRequest,
  code: string,
  message: string,
  trace_id: string = traceId(),
  receipts: ReceiptPointer[] = [],
  data: TData = emptyData<TData>()
): SdkEnvelope<TData> {
  return {
    ok: false,
    operation: request.operation,
    trace_id,
    receipts,
    data,
    error: { code, message },
  };
}

export function createCliTransport(options: CliTransportOptions): InfringTransport {
  const command = String(options.command || '').trim();
  if (!command) {
    throw new Error('sdk_cli_transport_requires_command');
  }
  if (typeof options.args_for_operation !== 'function') {
    throw new Error('sdk_cli_transport_requires_args_for_operation');
  }
  const allowSyntheticFallback =
    options.allow_synthetic_fallback === true &&
    String(process.env.INFRING_SDK_ALLOW_SYNTHETIC_FALLBACK || '').trim() === '1';
  const processTransportRequested =
    options.allow_process_transport === true ||
    String(process.env.INFRING_SDK_ALLOW_PROCESS_TRANSPORT || '').trim() === '1';
  const activeReleaseChannel = releaseChannel({
    ...process.env,
    ...(options.env || {}),
  });
  const productionRelease = isProductionReleaseChannel(activeReleaseChannel);
  const allowProcessTransport = processTransportRequested && !productionRelease;
  return {
    async invoke<TData extends JsonValue = JsonValue>(
      request: InfringTransportRequest
    ): Promise<SdkEnvelope<TData>> {
      if (!allowProcessTransport) {
        const code = productionRelease
          ? 'process_transport_forbidden_in_production'
          : 'resident_transport_required';
        const message = productionRelease
          ? `CLI process transport is forbidden for release channel '${activeReleaseChannel}'; route through resident IPC transport.`
          : 'CLI process transport is disabled by default; route through resident IPC transport or set INFRING_SDK_ALLOW_PROCESS_TRANSPORT=1 for emergency fallback.';
        return failureEnvelope<TData>(request, code, message);
      }
      const args = options.args_for_operation(request);
      const proc = await runSpawn(
        command,
        args,
        options.cwd,
        { ...process.env, ...(options.env || {}) },
        Math.max(1000, Number(options.timeout_ms || 120000))
      );
      const parsed = parseJsonLine(String(proc.stdout || '')) || {};
      const resolvedTraceId = String(parsed.trace_id || traceId());
      const receipts = asStringArray(parsed.receipts).map((receiptId) => ({
        receipt_id: receiptId,
        issued_at: nowIso(),
      }));
      const hasParsedData = Object.prototype.hasOwnProperty.call(parsed, 'data');
      const parsedData = hasParsedData ? (parsed.data as TData) : undefined;
      if (Number(proc.status || 0) !== 0 || proc.error) {
        return failureEnvelope<TData>(
          request,
          'transport_exit_nonzero',
          `Transport command exited with status ${String(proc.status ?? 1)}${
            proc.error ? ` (${String(proc.error.message || proc.error)})` : ''
          }`,
          resolvedTraceId,
          receipts,
          parsedData || emptyData<TData>()
        );
      }
      if (!hasParsedData) {
        return failureEnvelope<TData>(
          request,
          'missing_transport_data',
          allowSyntheticFallback
            ? 'Synthetic fallback disabled for deterministic transport mode.'
            : 'Transport succeeded but did not return a data payload.',
          resolvedTraceId,
          receipts
        );
      }
      if (receipts.length === 0) {
        return failureEnvelope<TData>(
          request,
          'missing_transport_receipts',
          'Transport succeeded but did not return receipts; deterministic receipts are required.',
          resolvedTraceId
        );
      }
      const data = parsedData as TData;
      return {
        ok: true,
        operation: request.operation,
        trace_id: resolvedTraceId,
        receipts,
        data,
      };
    },
  };
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
      const data = seeded as TData;
      return envelope(request.operation, data, request.policy_refs);
    },
  };
}
