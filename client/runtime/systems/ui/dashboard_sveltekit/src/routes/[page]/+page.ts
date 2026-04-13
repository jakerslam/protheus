import { error } from '@sveltejs/kit';
import { getDashboardPage, legacyDashboardPages } from '$lib/dashboard';

export const prerender = true;

export function entries() {
  return legacyDashboardPages.map((page) => ({ page: page.key }));
}

export function load({ params }) {
  const target = getDashboardPage(params.page);
  if (!target || target.mode !== 'legacy') {
    throw error(404, 'dashboard_page_not_found');
  }
  return {
    page: target,
  };
}
