import { asRecord, requestJson, type JsonRecord } from '$lib/api';

export type DashboardInstalledSkillRow = {
  name: string;
  description: string;
  version: string;
  author: string;
  runtime: string;
  tools_count: number;
  enabled: boolean;
  tags: string[];
};

export type DashboardMarketplaceSkillRow = {
  slug: string;
  name: string;
  summary: string;
  author: string;
  downloads: number;
  installed: boolean;
  tags: string[];
};

export type DashboardMarketplaceBrowse = {
  items: DashboardMarketplaceSkillRow[];
  next_cursor: string;
};

export type DashboardMcpServerSnapshot = {
  configured: unknown[];
  connected: unknown[];
  total_configured: number;
  total_connected: number;
};

function normalizeMarketplaceRow(value: unknown): DashboardMarketplaceSkillRow {
  const row = asRecord(value);
  return {
    slug: String(row.slug || row.name || '').trim(),
    name: String(row.name || row.title || row.slug || '').trim(),
    summary: String(row.summary || row.description || '').trim(),
    author: String(row.author || row.owner || '').trim(),
    downloads: Number(row.downloads || row.installs || 0) || 0,
    installed: row.installed === true,
    tags: (Array.isArray(row.tags) ? row.tags : []).map((tag) => String(tag || '').trim()).filter(Boolean),
  };
}

export async function readInstalledSkills(): Promise<DashboardInstalledSkillRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/skills');
  const rows = Array.isArray(payload.skills) ? payload.skills : [];
  return rows
    .map((value) => asRecord(value))
    .filter((row) => String(row.name || '').trim())
    .map((row) => ({
      name: String(row.name || '').trim(),
      description: String(row.description || '').trim(),
      version: String(row.version || '').trim(),
      author: String(row.author || '').trim(),
      runtime: String(row.runtime || 'unknown').trim() || 'unknown',
      tools_count: Number(row.tools_count || 0) || 0,
      enabled: row.enabled !== false,
      tags: (Array.isArray(row.tags) ? row.tags : []).map((tag) => String(tag || '').trim()).filter(Boolean),
    }));
}

export async function readMcpServers(): Promise<DashboardMcpServerSnapshot> {
  const payload = await requestJson<JsonRecord>('GET', '/api/mcp/servers');
  return {
    configured: Array.isArray(payload.configured) ? payload.configured : [],
    connected: Array.isArray(payload.connected) ? payload.connected : [],
    total_configured: Number(payload.total_configured || 0) || 0,
    total_connected: Number(payload.total_connected || 0) || 0,
  };
}

export async function browseMarketplace(sort = 'trending', cursor = ''): Promise<DashboardMarketplaceBrowse> {
  const suffix = cursor ? `&cursor=${encodeURIComponent(cursor)}` : '';
  const payload = await requestJson<JsonRecord>('GET', `/api/clawhub/browse?sort=${encodeURIComponent(sort)}&limit=20${suffix}`);
  return {
    items: (Array.isArray(payload.items) ? payload.items : []).map(normalizeMarketplaceRow),
    next_cursor: String(payload.next_cursor || '').trim(),
  };
}

export async function searchMarketplace(query: string): Promise<DashboardMarketplaceSkillRow[]> {
  const payload = await requestJson<JsonRecord>('GET', `/api/clawhub/search?q=${encodeURIComponent(query)}&limit=20`);
  return (Array.isArray(payload.items) ? payload.items : []).map(normalizeMarketplaceRow);
}

export async function installMarketplaceSkill(slug: string): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', '/api/clawhub/install', { slug });
  return String(payload.name || slug || 'Skill installed').trim();
}

export async function uninstallSkill(name: string): Promise<string> {
  await requestJson('POST', '/api/skills/uninstall', { name });
  return `Removed ${name}`;
}

export async function createPromptSkill(input: { name: string; description: string; prompt_context: string }): Promise<string> {
  await requestJson('POST', '/api/skills/create', {
    name: input.name,
    description: input.description,
    runtime: 'prompt_only',
    prompt_context: input.prompt_context,
  });
  return `Created ${input.name}`;
}
