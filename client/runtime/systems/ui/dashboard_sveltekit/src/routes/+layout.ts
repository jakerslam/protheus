import type { LayoutLoad } from './$types';
import {
  buildDashboardRuntimeConfig,
  toPublicDashboardRuntimeConfig
} from '$lib/dashboard_runtime_config';

export const ssr = true;
export const prerender = false;

export const load: LayoutLoad = async () => {
  const runtimeConfig = toPublicDashboardRuntimeConfig(buildDashboardRuntimeConfig());
  return {
    runtime: runtimeConfig,
    loadedAt: new Date().toISOString()
  };
};
