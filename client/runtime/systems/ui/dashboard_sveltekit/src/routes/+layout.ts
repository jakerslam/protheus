import type { LayoutLoad } from './$types';
import { buildRuntimeConfig, toPublicRuntimeConfig } from '$lib/runtime';

export const ssr = true;
export const prerender = false;

export const load: LayoutLoad = async () => {
  const runtime = toPublicRuntimeConfig(buildRuntimeConfig());
  return {
    runtime,
    loadedAt: new Date().toISOString()
  };
};
