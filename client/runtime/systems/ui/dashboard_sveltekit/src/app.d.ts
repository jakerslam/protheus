import type { DashboardRuntimePublicConfig } from '$lib/runtime';

declare global {
  namespace App {
    interface Locals {
      dashboardTraceId?: string;
    }
    interface PageData {
      runtime: DashboardRuntimePublicConfig;
      loadedAt: string;
    }
  }
}

export {};
