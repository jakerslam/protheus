export type RuntimeStatus = {
  ok?: boolean;
  connected?: boolean;
  daemon?: string;
  error?: string;
};

export async function readRuntimeStatus(): Promise<RuntimeStatus> {
  const response = await fetch('/api/status', { cache: 'no-store' });
  const payload = (await response.json()) as RuntimeStatus;
  return payload || {};
}

