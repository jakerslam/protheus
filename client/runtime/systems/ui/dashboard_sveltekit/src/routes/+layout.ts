import type { LayoutLoad } from './$types';
import { buildRuntimeConfig, pickPublicRuntimeConfig } from '$lib/runtime';

export const ssr = true;
export const prerender = false;

export const load: LayoutLoad = async () => {
  const runtime = pickPublicRuntimeConfig(buildRuntimeConfig());
  return {
    runtime,
    loadedAt: new Date().toISOString()
  };
};
