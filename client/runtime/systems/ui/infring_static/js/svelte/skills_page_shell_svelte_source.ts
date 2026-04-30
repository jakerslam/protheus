const COMPONENT_TAG = 'infring-skills-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-skills-page-shell', shadow: 'none' }} />
<script>
  import { onMount } from 'svelte';

  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'skills';
  export let panelRole = 'page';
  export let routeContract = 'skills';
  export let parentOwnedData = false;

  let view = {
    tab: 'installed',
    skills: [],
    loading: true,
    loadError: '',
    clawhubSearch: '',
    clawhubResults: [],
    clawhubBrowseResults: [],
    clawhubLoading: false,
    clawhubError: '',
    clawhubSort: 'trending',
    clawhubNextCursor: null,
    installingSlug: null,
    installResult: null,
    skillDetail: null,
    detailLoading: false,
    showSkillCode: false,
    skillCode: '',
    skillCodeFilename: '',
    skillCodeLoading: false,
    mcpServers: { configured: [], connected: [], total_configured: 0, total_connected: 0 },
    mcpLoading: false,
    categories: [],
    quickStartSkills: []
  };

  function hydrateLegacyViewModel() {
    if (typeof window === 'undefined' || typeof window.skillsPage !== 'function') return;
    view = window.skillsPage();
  }

  function repaint() {
    view = view;
  }

  async function call(methodName, ...args) {
    if (!view || typeof view[methodName] !== 'function') return undefined;
    var result = view[methodName].apply(view, args);
    repaint();
    if (result && typeof result.then === 'function') await result;
    repaint();
    return result;
  }

  function navigate(target) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(target);
    else if (typeof window !== 'undefined') window.location.hash = target;
  }

  function setTab(tab) {
    view.tab = tab;
    repaint();
    if (tab === 'clawhub' && !view.clawhubBrowseResults.length && !view.clawhubSearch) call('browseClawHub', 'trending');
    if (tab === 'mcp') call('loadMcpServers');
  }

  function setSearch(value) {
    view.clawhubSearch = value;
    repaint();
    call('onSearchInput');
  }

  function runtimeBadge(skill) {
    return view.runtimeBadge ? view.runtimeBadge(skill && skill.runtime) : { text: 'APP', cls: 'runtime-badge-prompt' };
  }

  function sourceBadge(skill) {
    return view.sourceBadge ? view.sourceBadge(skill && skill.source) : { text: 'Local', cls: 'badge-dim' };
  }

  function downloads(value) {
    return view.formatDownloads ? view.formatDownloads(value) : String(value || 0);
  }

  function installed(slug) {
    return view.isSkillInstalled ? view.isSkillInstalled(slug) : false;
  }

  function installedByName(name) {
    return view.isSkillInstalledByName ? view.isSkillInstalledByName(name) : false;
  }

  function mcpTotal(field) {
    var source = view.mcpServers && typeof view.mcpServers === 'object' ? view.mcpServers : {};
    return Number(source[field] || 0);
  }

  function mcpRows(field) {
    var source = view.mcpServers && typeof view.mcpServers === 'object' ? view.mcpServers : {};
    return Array.isArray(source[field]) ? source[field] : [];
  }

  onMount(async function() {
    hydrateLegacyViewModel();
    await call('loadSkills');
  });
</script>

<div>
  <div class="page-header">
    <div class="tabs mt-3" role="tablist">
      <div class="tab active" role="tab" on:click={() => navigate('skills')}>Apps</div>
      <div class="tab" role="tab" on:click={() => navigate('channels')}>Channels</div>
      <div class="tab" role="tab" on:click={() => navigate('eyes')}>Eyes</div>
      <div class="tab" role="tab" on:click={() => navigate('hands')}>Hands</div>
    </div>
  </div>
  <div class="page-body">
    <div class="info-card">
      <h4>Plugins &amp; Ecosystem</h4>
      <p>Plugins extend your agents with new capabilities. Infring supports the <strong>Infring/ClawHub</strong> ecosystem (3,000+ community plugins) plus local plugins.</p>
      <ul>
        <li><strong>Prompt-only</strong> &mdash; inject context and instructions into the agent's system prompt.</li>
        <li><strong>Python / Node.js</strong> &mdash; executable tools that agents can call during conversations.</li>
        <li><strong>MCP Servers</strong> &mdash; external tools via Model Context Protocol.</li>
      </ul>
    </div>

    {#if view.loading}
      <div class="loading-state"><div class="spinner"></div><span>Loading plugins...</span></div>
    {:else if view.loadError}
      <div class="error-state">
        <span class="error-icon">!</span>
        <p>{view.loadError}</p>
        <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('loadData')}>Retry</button>
      </div>
    {:else}
      <div class="tabs" role="tablist">
        <div class:active={view.tab === 'installed'} class="tab" role="tab" on:click={() => setTab('installed')}>
          Installed {#if view.skills.length}<span class="badge badge-dim" style="margin-left:4px">{view.skills.length}</span>{/if}
        </div>
        <div class:active={view.tab === 'clawhub'} class="tab" role="tab" on:click={() => setTab('clawhub')}>ClawHub</div>
        <div class:active={view.tab === 'mcp'} class="tab" role="tab" on:click={() => setTab('mcp')}>MCP Servers</div>
        <div class:active={view.tab === 'create'} class="tab" role="tab" on:click={() => setTab('create')}>Quick Start</div>
      </div>

      {#if view.tab === 'installed'}
        {#if view.skills.length}
          <div class="card-grid">
            {#each view.skills as skill (skill.name)}
              <div class="card">
                <div class="flex justify-between items-center mb-2">
                  <div class="flex items-center gap-2">
                    <div class="card-header" style="margin:0">{skill.name}</div>
                    <span class={'runtime-badge ' + runtimeBadge(skill).cls}>{runtimeBadge(skill).text}</span>
                    <span class={'badge ' + sourceBadge(skill).cls} style="font-size:0.65rem">{sourceBadge(skill).text}</span>
                  </div>
                  <div class:active={skill.enabled} class="toggle" on:click={() => { skill.enabled = !skill.enabled; repaint(); }}></div>
                </div>
                <div class="card-meta">{skill.description}</div>
                <div class="flex justify-between items-center mt-2">
                  <div class="flex gap-2 items-center">
                    <span class="text-xs text-dim">{skill.tools_count} tool(s)</span>
                    {#if skill.version}<span class="text-xs text-dim">v{skill.version}</span>{/if}
                    {#if skill.has_prompt_context}<span class="text-xs text-dim">(prompt context)</span>{/if}
                  </div>
                  <button class="btn btn-danger btn-sm" type="button" on:click={() => call('uninstallSkill', skill.name)}>Uninstall</button>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <infring-chat-stream-shell class="empty-state">
            <div class="empty-state-icon"><svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m16 18 6-6-6-6M8 6l-6 6 6 6"/></svg></div>
            <h3>No plugins installed</h3>
            <p>Plugins add new capabilities to your agents. Browse ClawHub for community plugins or create your own.</p>
            <div class="flex gap-2">
              <button class="btn btn-primary" type="button" on:click={() => setTab('clawhub')}>Browse ClawHub</button>
              <button class="btn btn-ghost" type="button" on:click={() => setTab('create')}>Quick Start</button>
            </div>
          </infring-chat-stream-shell>
        {/if}
      {:else if view.tab === 'clawhub'}
        <div class="search-input mb-4" style="position:relative">
          <span style="color:var(--text-muted)"><svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg></span>
          <input placeholder="Search ClawHub plugins... (type to search)" value={view.clawhubSearch} on:input={(event) => setSearch(event.currentTarget.value)} on:keydown={(event) => { if (event.key === 'Enter' && !event.isComposing) call('searchClawHub'); if (event.key === 'Escape') call('clearSearch'); }}>
          {#if view.clawhubSearch}<button class="search-clear-btn" title="Clear search (Esc)" type="button" on:click={() => call('clearSearch')}>&times;</button>{/if}
        </div>
        {#if !view.clawhubSearch}
          <div class="filter-pills mb-4">
            {#each ['trending', 'downloads', 'stars', 'updated'] as sort}
              <span class:active={view.clawhubSort === sort} class="filter-pill" on:click={() => call('browseClawHub', sort)}>{sort === 'downloads' ? 'Most Downloaded' : sort === 'stars' ? 'Most Starred' : sort === 'updated' ? 'Recently Updated' : 'Trending'}</span>
            {/each}
          </div>
          <div class="mb-4">
            <div class="text-xs text-dim mb-2" style="letter-spacing:0.5px">CATEGORIES</div>
            <div class="flex flex-wrap gap-1">
              {#each view.categories as cat (cat.id)}
                <span class="filter-pill" style="font-size:0.7rem;padding:2px 8px" on:click={() => call('searchCategory', cat)}>{cat.name}</span>
              {/each}
            </div>
          </div>
        {/if}
        {#if view.clawhubLoading}
          <div class="loading-state"><div class="spinner"></div><span>{view.clawhubSearch ? 'Searching ClawHub...' : 'Loading plugins...'}</span></div>
        {/if}
        {#if view.clawhubError && !view.clawhubLoading}
          <div class="error-state">
            <span class="error-icon">!</span>
            <p>{view.clawhubError}</p>
            <p class="hint mt-2">ClawHub may be temporarily unavailable. The Infring ecosystem is hosted at clawhub.ai.</p>
            <button class="btn btn-ghost btn-sm mt-2" type="button" on:click={() => view.clawhubSearch ? call('searchClawHub') : call('browseClawHub', view.clawhubSort)}>Retry</button>
          </div>
        {/if}
        {#if view.clawhubSearch && view.clawhubResults.length && !view.clawhubLoading}
          <div class="flex justify-between items-center mb-3">
            <div class="text-sm text-dim">{view.clawhubResults.length} result(s) for "{view.clawhubSearch}"</div>
            <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('clearSearch')}>Clear search</button>
          </div>
          <div class="card-grid">
            {#each view.clawhubResults as skill (skill.slug)}
              <div class="card card-glow" style="cursor:pointer" on:click={() => call('showSkillDetail', skill.slug)}>
                <div class="flex justify-between items-center mb-2">
                  <div class="card-header" style="margin:0">{skill.name || skill.slug}</div>
                  <span class="badge badge-info" style="font-size:0.6rem">ClawHub</span>
                </div>
                <div class="card-meta" style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden">{skill.description}</div>
                <div class="flex justify-between items-center mt-3">
                  <div class="flex gap-3 items-center">{#if skill.version}<span class="text-xs text-dim">v{skill.version}</span>{/if}</div>
                  <button class="btn btn-primary btn-sm" type="button" on:click|stopPropagation={() => call('installFromClawHub', skill.slug)} disabled={view.installingSlug === skill.slug || skill.installed || installed(skill.slug)}>{skill.installed || installed(skill.slug) ? 'Installed' : view.installingSlug === skill.slug ? 'Installing...' : 'Install'}</button>
                </div>
              </div>
            {/each}
          </div>
        {:else if !view.clawhubSearch && view.clawhubBrowseResults.length && !view.clawhubLoading}
          <div class="card-grid">
            {#each view.clawhubBrowseResults as skill (skill.slug)}
              <div class="card card-glow" style="cursor:pointer" on:click={() => call('showSkillDetail', skill.slug)}>
                <div class="flex justify-between items-center mb-2">
                  <div class="card-header" style="margin:0">{skill.name || skill.slug}</div>
                  <span class="badge badge-info" style="font-size:0.6rem">ClawHub</span>
                </div>
                <div class="card-meta" style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden">{skill.description}</div>
                <div class="flex justify-between items-center mt-3">
                  <div class="flex gap-3 items-center">
                    {#if skill.downloads}<span class="text-xs text-dim">{downloads(skill.downloads)} downloads</span>{/if}
                    {#if skill.stars}<span class="text-xs text-dim">{skill.stars} stars</span>{/if}
                    {#if skill.version}<span class="text-xs text-dim">v{skill.version}</span>{/if}
                  </div>
                  <button class="btn btn-primary btn-sm" type="button" on:click|stopPropagation={() => call('installFromClawHub', skill.slug)} disabled={view.installingSlug === skill.slug || skill.installed || installed(skill.slug)}>{skill.installed || installed(skill.slug) ? 'Installed' : view.installingSlug === skill.slug ? 'Installing...' : 'Install'}</button>
                </div>
              </div>
            {/each}
          </div>
          {#if view.clawhubNextCursor}<div class="text-center mt-4"><button class="btn btn-ghost" type="button" on:click={() => call('loadMoreClawHub')} disabled={view.clawhubLoading}>Load More</button></div>{/if}
        {:else if view.clawhubSearch && !view.clawhubLoading && !view.clawhubError}
          <infring-chat-stream-shell class="empty-state">
            <p>No plugins found for "{view.clawhubSearch}"</p>
            <p class="hint mt-1">Try a different search term or browse by category.</p>
            <button class="btn btn-ghost btn-sm mt-2" type="button" on:click={() => call('clearSearch')}>Back to browse</button>
          </infring-chat-stream-shell>
        {/if}
      {:else if view.tab === 'mcp'}
        <div class="info-card">
          <h4>MCP Servers (Model Context Protocol)</h4>
          <p>MCP servers provide external tools to your agents &mdash; GitHub, filesystem, databases, APIs, and more.</p>
          <p class="mt-2" style="font-size:0.8rem;color:var(--text-dim)">Configure MCP servers in your <code>config.toml</code> under <code>[mcp_servers]</code>.</p>
        </div>
        {#if view.mcpLoading}
          <div class="loading-state"><div class="spinner"></div><span>Loading MCP servers...</span></div>
        {:else}
          {#if mcpTotal('total_connected') > 0}
            <div class="mb-4">
              <div class="text-sm font-bold mb-2" style="color:var(--text-dim);letter-spacing:0.5px">CONNECTED SERVERS</div>
              <div class="card-grid">
                {#each mcpRows('connected') as srv (srv.name)}
                  <div class="card">
                    <div class="flex justify-between items-center mb-2"><div class="card-header" style="margin:0">{srv.name}</div><span class="badge badge-success">Connected</span></div>
                    <div class="card-meta">{srv.tools_count} tool(s) available</div>
                    {#if srv.tools && srv.tools.length}
                      <div class="mt-2">
                        <div class="text-xs text-dim mb-1">Tools:</div>
                        {#each srv.tools.slice(0, 10) as tool (tool.name)}
                          <div class="text-xs" style="padding:2px 0"><code style="font-size:0.7rem">{tool.name}</code>{#if tool.description}<span class="text-dim" style="font-size:0.65rem"> - {tool.description}</span>{/if}</div>
                        {/each}
                        {#if srv.tools.length > 10}<div class="text-xs text-dim">... and {srv.tools.length - 10} more</div>{/if}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
          {#if mcpTotal('total_configured') > 0}
            <div class="mb-4">
              <div class="text-sm font-bold mb-2" style="color:var(--text-dim);letter-spacing:0.5px">CONFIGURED SERVERS</div>
              <div class="card-grid">
                {#each mcpRows('configured') as srv (srv.name)}
                  <div class="card card-unconfigured">
                    <div class="flex justify-between items-center mb-2"><div class="card-header" style="margin:0">{srv.name}</div><span class="badge badge-dim">{srv.transport && srv.transport.type}</span></div>
                    {#if srv.transport && srv.transport.type === 'stdio'}<div class="text-xs"><code style="font-size:0.7rem">{srv.transport.command} {(srv.transport.args || []).join(' ')}</code></div>{/if}
                    {#if srv.transport && srv.transport.type === 'sse'}<div class="text-xs"><code style="font-size:0.7rem">{srv.transport.url}</code></div>{/if}
                    {#if srv.env && srv.env.length}<div class="text-xs text-dim mt-1">Env: {srv.env.join(', ')}</div>{/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
          {#if mcpTotal('total_configured') === 0 && mcpTotal('total_connected') === 0}
            <infring-chat-stream-shell class="empty-state">
              <h4>No MCP servers configured</h4>
              <p class="hint">MCP servers extend your agents with external tools. Add servers to your config.toml.</p>
            </infring-chat-stream-shell>
          {/if}
        {/if}
      {:else if view.tab === 'create'}
        <div class="info-card">
          <h4>Quick Start Plugins</h4>
          <p>Create prompt-only plugins with one click. These inject context into your agent's system prompt &mdash; no code required.</p>
        </div>
        <div class="card-grid">
          {#each view.quickStartSkills as qs (qs.name)}
            <div class="card card-glow">
              <div class="flex justify-between items-center mb-2">
                <div class="card-header" style="margin:0">{qs.name}</div>
                <span class="runtime-badge runtime-badge-prompt">PROMPT</span>
              </div>
              <div class="card-meta">{qs.description}</div>
              <div class="flex justify-end mt-2">
                <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('createDemoSkill', qs)} disabled={installedByName(qs.name)}>{installedByName(qs.name) ? 'Created' : 'Create Plugin'}</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>

{#if view.skillDetail || view.detailLoading}
  <div class="modal-overlay" on:click|self={() => call('closeDetail')}>
    <div class="modal" style="max-width:600px">
      {#if view.detailLoading}
        <div class="loading-state" style="padding:40px 0"><div class="spinner"></div><span>Loading plugin details...</span></div>
      {:else if view.skillDetail}
        <div class="modal-header"><h3>{view.skillDetail.name || view.skillDetail.slug}</h3><button class="modal-close" type="button" on:click={() => call('closeDetail')}>&times;</button></div>
        <div class="mb-3">
          <div class="flex gap-2 items-center flex-wrap">
            <span class="badge badge-info">ClawHub</span>
            {#if view.skillDetail.version}<span class="text-xs text-dim">v{view.skillDetail.version}</span>{/if}
            {#if view.skillDetail.author_name || view.skillDetail.author}<span class="text-xs" style="color:var(--text-dim)">by {view.skillDetail.author_name || view.skillDetail.author}</span>{/if}
          </div>
        </div>
        <div class="flex gap-4 items-center mb-3">
          {#if view.skillDetail.downloads}<span class="text-sm" style="color:var(--text-dim)">{downloads(view.skillDetail.downloads)} downloads</span>{/if}
          {#if view.skillDetail.stars}<span class="text-sm" style="color:var(--text-dim)">{view.skillDetail.stars} stars</span>{/if}
        </div>
        {#if view.skillDetail.description}<div class="mb-4"><p>{view.skillDetail.description}</p></div>{/if}
        {#if view.skillDetail.tags && typeof view.skillDetail.tags === 'object'}
          <div class="mb-4"><div class="flex flex-wrap gap-1">{#each Object.keys(view.skillDetail.tags || {}) as key}<span class="category-badge">{key}: {view.skillDetail.tags[key]}</span>{/each}</div></div>
        {/if}
        {#if view.installResult && view.installResult.warnings && view.installResult.warnings.length}
          <div class="mb-4">
            <div class="form-group"><label>Security Warnings</label></div>
            {#each view.installResult.warnings as warning (warning.message)}
              <div class={warning.severity === 'Critical' ? 'text-xs text-danger' : 'text-xs text-dim'} style="padding:2px 0">[{warning.severity}] {warning.message}</div>
            {/each}
          </div>
        {/if}
        <div class="flex gap-2">
          <button class="btn btn-primary" style="flex:1" type="button" on:click={() => call('installFromClawHub', view.skillDetail.slug)} disabled={view.installingSlug === view.skillDetail.slug || view.skillDetail.installed || installed(view.skillDetail.slug)}>{view.skillDetail.installed || installed(view.skillDetail.slug) ? 'Already Installed' : view.installingSlug === view.skillDetail.slug ? 'Installing...' : 'Install from ClawHub'}</button>
          <button class="btn btn-ghost" type="button" on:click={() => call('viewSkillCode', view.skillDetail.slug)} disabled={view.skillCodeLoading}>{view.skillCodeLoading ? 'Loading...' : view.showSkillCode ? 'Hide Code' : 'View Code'}</button>
        </div>
        {#if view.showSkillCode && view.skillCode}
          <div class="mt-3" style="max-height:300px;overflow:auto;border:1px solid var(--border);border-radius:8px;background:var(--bg-inset)">
            <div class="flex justify-between items-center" style="padding:6px 12px;border-bottom:1px solid var(--border)"><span class="text-xs text-dim">{view.skillCodeFilename}</span></div>
            <pre style="margin:0;padding:12px;font-size:12px;line-height:1.5;white-space:pre-wrap;word-break:break-all">{view.skillCode}</pre>
          </div>
        {/if}
        <div class="text-xs text-dim mt-2 text-center">Plugins are security-scanned before installation. Prompt injection and malware patterns are blocked.</div>
      {/if}
    </div>
  </div>
{/if}

`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
