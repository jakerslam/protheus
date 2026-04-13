export type DashboardPageMode = 'native' | 'legacy';
export type DashboardPageChurn = 'high' | 'medium' | 'low';

export type DashboardPageKey =
  | 'overview'
  | 'chat'
  | 'agents'
  | 'sessions'
  | 'approvals'
  | 'comms'
  | 'workflows'
  | 'scheduler'
  | 'channels'
  | 'eyes'
  | 'skills'
  | 'hands'
  | 'analytics'
  | 'logs'
  | 'runtime'
  | 'settings'
  | 'wizard';

export type DashboardPage = {
  key: DashboardPageKey;
  title: string;
  summary: string;
  mode: DashboardPageMode;
  churn: DashboardPageChurn;
};

export const dashboardPages: DashboardPage[] = [
  { key: 'overview', title: 'Overview', summary: 'Primary SvelteKit landing view and migration dashboard.', mode: 'native', churn: 'medium' },
  { key: 'chat', title: 'Conversations', summary: 'High-churn chat workspace with live agent traffic.', mode: 'native', churn: 'high' },
  { key: 'agents', title: 'Agents', summary: 'Agent roster, creation, and lifecycle management.', mode: 'legacy', churn: 'high' },
  { key: 'sessions', title: 'Memory', summary: 'Session history, continuity, and recall surfaces.', mode: 'legacy', churn: 'medium' },
  { key: 'approvals', title: 'Approvals', summary: 'Queued approvals and gated operator actions.', mode: 'legacy', churn: 'medium' },
  { key: 'comms', title: 'Comms', summary: 'Connected channels and outbound message surfaces.', mode: 'legacy', churn: 'medium' },
  { key: 'workflows', title: 'Workflows', summary: 'Workflow definitions, orchestration, and runs.', mode: 'legacy', churn: 'high' },
  { key: 'scheduler', title: 'Scheduler', summary: 'Automations, cadence, and heartbeat timing.', mode: 'legacy', churn: 'medium' },
  { key: 'channels', title: 'Channels', summary: 'External messaging and inbound/outbound connectors.', mode: 'legacy', churn: 'medium' },
  { key: 'eyes', title: 'Eyes', summary: 'Visual sensing, receipts, and manual capture lanes.', mode: 'legacy', churn: 'medium' },
  { key: 'skills', title: 'Skills', summary: 'Installed skill inventory and capability browser.', mode: 'legacy', churn: 'medium' },
  { key: 'hands', title: 'Hands', summary: 'Manual actions, desktop control, and actuation status.', mode: 'legacy', churn: 'medium' },
  { key: 'analytics', title: 'Analytics', summary: 'Usage, spend, and per-model breakdowns.', mode: 'legacy', churn: 'medium' },
  { key: 'logs', title: 'Logs', summary: 'Recent audit activity and operator-facing receipts.', mode: 'legacy', churn: 'medium' },
  { key: 'runtime', title: 'Runtime', summary: 'Backend health, providers, and web tooling status.', mode: 'legacy', churn: 'high' },
  { key: 'settings', title: 'Settings', summary: 'Provider config, auth, and dashboard behavior.', mode: 'legacy', churn: 'high' },
  { key: 'wizard', title: 'Wizard', summary: 'Guided setup and onboarding utilities.', mode: 'legacy', churn: 'low' },
];

const pageMap = new Map<DashboardPageKey, DashboardPage>(
  dashboardPages.map((page) => [page.key, page])
);

export const legacyDashboardPages = dashboardPages.filter((page) => page.mode === 'legacy');
export const nativeDashboardPages = dashboardPages.filter((page) => page.mode === 'native');
export const highChurnMigrationTargets = legacyDashboardPages.filter((page) => page.churn === 'high');

export function getDashboardPage(key: string | null | undefined): DashboardPage | null {
  const normalized = String(key || '').trim().toLowerCase() as DashboardPageKey;
  return pageMap.get(normalized) || null;
}

export function dashboardPageHref(key: DashboardPageKey): string {
  return key === 'overview' ? '/dashboard/overview' : `/dashboard/${key}`;
}

export function dashboardClassicHref(key?: DashboardPageKey | null): string {
  return key ? `/dashboard-classic#${encodeURIComponent(key)}` : '/dashboard-classic';
}

export function dashboardEmbeddedFallbackHref(key: DashboardPageKey): string {
  return `/dashboard-classic?embed=1&page=${encodeURIComponent(key)}#${encodeURIComponent(key)}`;
}

export function resolveDashboardPageFromPathname(pathname: string | null | undefined): DashboardPage {
  const normalized = String(pathname || '').replace(/\/+$/, '') || '/dashboard';
  if (normalized === '/' || normalized === '/dashboard' || normalized === '/dashboard/') {
    return pageMap.get('overview')!;
  }
  const withoutBase = normalized.startsWith('/dashboard/')
    ? normalized.slice('/dashboard/'.length)
    : normalized.replace(/^\/+/, '');
  return getDashboardPage(withoutBase) || pageMap.get('overview')!;
}
