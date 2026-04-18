import { browser } from '$app/environment';

export interface DashboardRuntimeConfig {
  apiBaseUrl: string;
  wsBaseUrl: string;
  authStorageKey: string;
  snapshotPollMs: number;
  streamReconnectMs: number;
  telemetryEnabled: boolean;
  featureFlags: Record<string, boolean>;
}

export interface DashboardRuntimePublicConfig {
  apiBaseUrl: string;
  wsBaseUrl: string;
  snapshotPollMs: number;
  streamReconnectMs: number;
  telemetryEnabled: boolean;
  featureFlags: Record<string, boolean>;
}

type RuntimeEnv = Record<string, string | undefined>;

const DEFAULT_SNAPSHOT_POLL_MS = 3_000;
const DEFAULT_STREAM_RECONNECT_MS = 1_500;
const DEFAULT_AUTH_STORAGE_KEY = 'infring-api-key';

function normalizeInteger(value: unknown, fallback: number, min: number, max: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  const rounded = Math.round(parsed);
  if (rounded < min) return min;
  if (rounded > max) return max;
  return rounded;
}

function normalizeBoolean(value: unknown, fallback: boolean): boolean {
  if (typeof value === 'boolean') return value;
  const lowered = String(value ?? '').trim().toLowerCase();
  if (!lowered) return fallback;
  if (lowered === '1' || lowered === 'true' || lowered === 'yes' || lowered === 'on') return true;
  if (lowered === '0' || lowered === 'false' || lowered === 'no' || lowered === 'off') return false;
  return fallback;
}

function normalizeBaseUrl(rawValue: unknown, fallback: string): string {
  const raw = String(rawValue ?? '').trim();
  if (!raw) return fallback;
  try {
    const parsed = new URL(raw);
    return parsed.toString().replace(/\/+$/, '');
  } catch (_) {
    return fallback;
  }
}

function inferApiBaseFromLocation(): string {
  if (!browser) return 'http://127.0.0.1:8200';
  const locationOrigin = String(window.location.origin || '').trim();
  if (!locationOrigin) return 'http://127.0.0.1:8200';
  return locationOrigin.replace(/\/+$/, '');
}

function inferWsBase(apiBaseUrl: string): string {
  try {
    const parsed = new URL(apiBaseUrl);
    parsed.protocol = parsed.protocol === 'https:' ? 'wss:' : 'ws:';
    return parsed.toString().replace(/\/+$/, '');
  } catch (_) {
    return 'ws://127.0.0.1:8200';
  }
}

function parseFeatureFlags(rawValue: unknown): Record<string, boolean> {
  const out: Record<string, boolean> = {};
  const raw = String(rawValue ?? '').trim();
  if (!raw) return out;
  for (const token of raw.split(',')) {
    const key = String(token || '').trim().toLowerCase();
    if (!key) continue;
    out[key] = true;
  }
  return out;
}

function readRuntimeEnv(): RuntimeEnv {
  const out: RuntimeEnv = {};
  const metaEnv =
    typeof import.meta !== 'undefined' &&
    (import.meta as unknown as { env?: Record<string, unknown> }).env &&
    typeof (import.meta as unknown as { env?: Record<string, unknown> }).env === 'object'
      ? ((import.meta as unknown as { env: Record<string, unknown> }).env as Record<string, unknown>)
      : {};
  const processEnv =
    typeof process !== 'undefined' &&
    process &&
    typeof process === 'object' &&
    process.env &&
    typeof process.env === 'object'
      ? (process.env as Record<string, string | undefined>)
      : {};

  const keys = [
    'VITE_INFRING_API_BASE_URL',
    'VITE_INFRING_WS_BASE_URL',
    'VITE_INFRING_AUTH_STORAGE_KEY',
    'VITE_INFRING_SNAPSHOT_POLL_MS',
    'VITE_INFRING_STREAM_RECONNECT_MS',
    'VITE_INFRING_TELEMETRY_ENABLED',
    'VITE_INFRING_FEATURE_FLAGS'
  ];
  for (const key of keys) {
    const metaValue = metaEnv[key];
    if (metaValue != null) {
      out[key] = String(metaValue);
      continue;
    }
    out[key] = processEnv[key];
  }
  return out;
}

export function buildRuntimeConfig(env: RuntimeEnv = readRuntimeEnv()): DashboardRuntimeConfig {
  const inferredApiBase = inferApiBaseFromLocation();
  const apiBaseUrl = normalizeBaseUrl(env.VITE_INFRING_API_BASE_URL, inferredApiBase);
  const wsBaseUrl = normalizeBaseUrl(env.VITE_INFRING_WS_BASE_URL, inferWsBase(apiBaseUrl));
  return {
    apiBaseUrl,
    wsBaseUrl,
    authStorageKey: String(env.VITE_INFRING_AUTH_STORAGE_KEY || DEFAULT_AUTH_STORAGE_KEY),
    snapshotPollMs: normalizeInteger(env.VITE_INFRING_SNAPSHOT_POLL_MS, DEFAULT_SNAPSHOT_POLL_MS, 250, 120_000),
    streamReconnectMs: normalizeInteger(
      env.VITE_INFRING_STREAM_RECONNECT_MS,
      DEFAULT_STREAM_RECONNECT_MS,
      100,
      60_000
    ),
    telemetryEnabled: normalizeBoolean(env.VITE_INFRING_TELEMETRY_ENABLED, true),
    featureFlags: parseFeatureFlags(env.VITE_INFRING_FEATURE_FLAGS)
  };
}

export function pickPublicRuntimeConfig(config: DashboardRuntimeConfig): DashboardRuntimePublicConfig {
  return {
    apiBaseUrl: config.apiBaseUrl,
    wsBaseUrl: config.wsBaseUrl,
    snapshotPollMs: config.snapshotPollMs,
    streamReconnectMs: config.streamReconnectMs,
    telemetryEnabled: config.telemetryEnabled,
    featureFlags: Object.assign({}, config.featureFlags)
  };
}
