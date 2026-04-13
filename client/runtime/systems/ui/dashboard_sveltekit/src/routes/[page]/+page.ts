import { error } from '@sveltejs/kit';
import { getDashboardPage, legacyDashboardPages } from '$lib/dashboard';

export const prerender = false;

export function load({ params }) {
  const target = getDashboardPage(params.page);
  if (!target || target.mode !== 'legacy') {
    throw error(404, 'dashboard_page_not_found');
  }
  return {
    page: target,
  };
}
