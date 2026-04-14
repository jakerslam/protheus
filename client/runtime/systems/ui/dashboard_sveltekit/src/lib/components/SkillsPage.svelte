<script lang="ts">
  import { browseMarketplace, createPromptSkill, installMarketplaceSkill, readInstalledSkills, readMcpServers, searchMarketplace, uninstallSkill, type DashboardInstalledSkillRow, type DashboardMarketplaceSkillRow, type DashboardMcpServerSnapshot } from '$lib/skills';
  import { onMount } from 'svelte';

  let installed: DashboardInstalledSkillRow[] = [];
  let marketplace: DashboardMarketplaceSkillRow[] = [];
  let mcp: DashboardMcpServerSnapshot = { configured: [], connected: [], total_configured: 0, total_connected: 0 };
  let search = '';
  let sort = 'trending';
  let nextCursor = '';
  let createName = '';
  let createDescription = '';
  let createPrompt = '';
  let loading = true;
  let busyKey = '';
  let error = '';
  let notice = '';

  onMount(async () => {
    await refresh();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      const [skillRows, mcpSnapshot, browse] = await Promise.all([
        readInstalledSkills(),
        readMcpServers(),
        browseMarketplace(sort),
      ]);
      installed = skillRows;
      mcp = mcpSnapshot;
      marketplace = browse.items;
      nextCursor = browse.next_cursor;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'skills_unavailable');
    } finally {
      loading = false;
    }
  }

  async function refreshMarketplace(): Promise<void> {
    if (!search.trim()) {
      const browse = await browseMarketplace(sort);
      marketplace = browse.items;
      nextCursor = browse.next_cursor;
      return;
    }
    marketplace = await searchMarketplace(search.trim());
    nextCursor = '';
  }

  async function loadMore(): Promise<void> {
    if (!nextCursor) return;
    busyKey = 'more';
    try {
      const browse = await browseMarketplace(sort, nextCursor);
      marketplace = marketplace.concat(browse.items);
      nextCursor = browse.next_cursor;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'skills_browse_failed');
    } finally {
      busyKey = '';
    }
  }

  async function submitSearch(): Promise<void> {
    busyKey = 'search';
    try {
      await refreshMarketplace();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'skills_search_failed');
    } finally {
      busyKey = '';
    }
  }

  async function installSkill(slug: string): Promise<void> {
    busyKey = `install:${slug}`;
    try {
      const label = await installMarketplaceSkill(slug);
      notice = `${label} installed`;
      installed = await readInstalledSkills();
      await refreshMarketplace();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'skill_install_failed');
    } finally {
      busyKey = '';
    }
  }

  async function removeSkill(name: string): Promise<void> {
    busyKey = `remove:${name}`;
    try {
      notice = await uninstallSkill(name);
      installed = await readInstalledSkills();
      await refreshMarketplace();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'skill_remove_failed');
    } finally {
      busyKey = '';
    }
  }

  async function createSkill(): Promise<void> {
    if (!createName.trim() || !createPrompt.trim()) return;
    busyKey = 'create';
    try {
      notice = await createPromptSkill({
        name: createName.trim(),
        description: createDescription.trim(),
        prompt_context: createPrompt.trim(),
      });
      createName = '';
      createDescription = '';
      createPrompt = '';
      installed = await readInstalledSkills();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'skill_create_failed');
    } finally {
      busyKey = '';
    }
  }
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native skills</p>
      <h2>Installed skills, marketplace browse/search, and MCP server status in the Svelte shell.</h2>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {:else if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Installed</h3><span class="meta">{installed.length} skills</span></div>
      <div class="rows">
        {#each installed as skill}
          <div class="row">
            <div class="row-copy">
              <strong>{skill.name}</strong>
              <span>{skill.description || `${skill.runtime} runtime`}</span>
            </div>
            <div class="row-actions">
              <span>{skill.tools_count} tools</span>
              <button class="ghost small" type="button" disabled={busyKey === `remove:${skill.name}`} on:click={() => void removeSkill(skill.name)}>Uninstall</button>
            </div>
          </div>
        {/each}
      </div>
    </article>

    <article class="panel">
      <div class="panel-head"><h3>Create prompt-only skill</h3><span class="meta">Bounded native slice</span></div>
      <div class="form-grid">
        <input bind:value={createName} class="field" type="text" placeholder="Skill name" />
        <input bind:value={createDescription} class="field" type="text" placeholder="Description" />
        <textarea bind:value={createPrompt} class="field area" rows="5" placeholder="Prompt context"></textarea>
        <button class="primary small" type="button" disabled={busyKey === 'create'} on:click={() => void createSkill()}>{busyKey === 'create' ? 'Creating…' : 'Create skill'}</button>
      </div>
    </article>
  </div>

  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Marketplace</h3><span class="meta">{marketplace.length} results</span></div>
      <div class="hero-actions">
        <input bind:value={search} class="field" type="text" placeholder="Search marketplace…" />
        <select bind:value={sort} class="field narrow" on:change={() => void submitSearch()}>
          <option value="trending">Trending</option>
          <option value="downloads">Downloads</option>
          <option value="new">New</option>
        </select>
        <button class="ghost small" type="button" disabled={busyKey === 'search'} on:click={() => void submitSearch()}>{busyKey === 'search' ? 'Searching…' : 'Search'}</button>
      </div>
      <div class="rows">
        {#each marketplace as skill}
          <div class="row">
            <div class="row-copy">
              <strong>{skill.name}</strong>
              <span>{skill.summary || skill.slug}</span>
            </div>
            <div class="row-actions">
              <span>{skill.downloads} downloads</span>
              <button class="ghost small" type="button" disabled={skill.installed || busyKey === `install:${skill.slug}`} on:click={() => void installSkill(skill.slug)}>{skill.installed ? 'Installed' : 'Install'}</button>
            </div>
          </div>
        {/each}
      </div>
      {#if nextCursor && !search.trim()}
        <button class="ghost small" type="button" disabled={busyKey === 'more'} on:click={() => void loadMore()}>{busyKey === 'more' ? 'Loading…' : 'Load more'}</button>
      {/if}
    </article>

    <article class="panel">
      <div class="panel-head"><h3>MCP servers</h3><span class="meta">{mcp.total_connected}/{mcp.total_configured} connected</span></div>
      <div class="rows">
        <div class="row"><div class="row-copy"><strong>Configured servers</strong><span>Visible to the native skills page.</span></div><span>{mcp.total_configured}</span></div>
        <div class="row"><div class="row-copy"><strong>Connected servers</strong><span>Live connector health from the authoritative runtime.</span></div><span>{mcp.total_connected}</span></div>
      </div>
    </article>
  </div>
</section>

<style>
  .page, .grid, .rows, .form-grid { display: grid; gap: 18px; }
  .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero, .panel, .banner, .row, .field { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row, .row-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; background: rgba(255,255,255,0.04); }
  .row-copy { display: grid; gap: 4px; }
  .field { padding: 0.75rem 0.85rem; font: inherit; width: 100%; }
  .narrow { max-width: 140px; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; text-decoration: none; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, .meta, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .hero, .hero-actions, .row, .row-actions { flex-direction: column; align-items: flex-start; } }
</style>
