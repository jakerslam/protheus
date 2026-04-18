import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

function normalizeBoolean(value: unknown, fallback: boolean): boolean {
  if (typeof value === 'boolean') return value;
  const lowered = String(value ?? '').trim().toLowerCase();
  if (!lowered) return fallback;
  if (lowered === '1' || lowered === 'true' || lowered === 'yes' || lowered === 'on') return true;
  if (lowered === '0' || lowered === 'false' || lowered === 'no' || lowered === 'off') return false;
  return fallback;
}

function normalizePort(value: unknown, fallback: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  const rounded = Math.round(parsed);
  if (rounded < 1 || rounded > 65_535) return fallback;
  return rounded;
}

function normalizeHost(value: unknown, fallback: string): string {
  const host = String(value ?? '').trim();
  if (!host) return fallback;
  return host;
}

export default defineConfig(() => {
  const host = normalizeHost(process.env.INFRING_DASHBOARD_HOST, '127.0.0.1');
  const port = normalizePort(process.env.INFRING_DASHBOARD_PORT, 4173);
  const strictPort = normalizeBoolean(process.env.INFRING_DASHBOARD_STRICT_PORT, true);

  return {
    plugins: [sveltekit()],
    server: {
      host,
      port,
      strictPort
    },
    preview: {
      host,
      port: normalizePort(process.env.INFRING_DASHBOARD_PREVIEW_PORT, port + 100)
    },
    define: {
      __INFRING_BUILD_PROFILE__: JSON.stringify(process.env.INFRING_BUILD_PROFILE || 'dashboard_sveltekit')
    }
  };
});
