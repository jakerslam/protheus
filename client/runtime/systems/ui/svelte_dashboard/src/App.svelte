<script>
  import { onMount } from 'svelte';

  const MAX_CONNECT_ATTEMPTS = 5;
  const RETRY_BASE_DELAY_MS = 250;

  let page = 'chat';
  let connectionState = 'connecting';
  let loading = true;
  let refreshing = false;
  let lastError = '';

  let agents = [];
  let selectedAgentId = '';
  let messages = [];
  let composer = '';
  let sending = false;
  let creatingAgent = false;

  let channels = [];
  let channelsLoading = false;

  let skills = [];
  let marketplace = [];
  let skillsLoading = false;
  let skillsQuery = '';
  let installingSlug = '';

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  function cleanText(value) {
    return String(value == null ? '' : value);
  }

  async function apiRequest(path, options = {}) {
    const method = cleanText(options.method || 'GET').toUpperCase();
    const body = options.body == null ? null : options.body;
    const headers = { 'Content-Type': 'application/json' };
    let lastErr = null;
    for (let attempt = 1; attempt <= MAX_CONNECT_ATTEMPTS; attempt += 1) {
      try {
        const res = await fetch(path, {
          method,
          headers,
          body: body == null ? undefined : JSON.stringify(body),
          cache: 'no-store',
        });
        if (!res.ok) {
          const payloadText = await res.text();
          let serverError = '';
          try {
            const parsed = JSON.parse(payloadText);
            serverError = cleanText(parsed && (parsed.error || parsed.message || ''));
          } catch (_) {
            serverError = cleanText(payloadText);
          }
          if ((res.status === 502 || res.status === 503 || res.status === 504) && attempt < MAX_CONNECT_ATTEMPTS) {
            connectionState = 'reconnecting';
            await sleep(Math.min(1500, RETRY_BASE_DELAY_MS * attempt));
            continue;
          }
          throw new Error(serverError || `http_${res.status}`);
        }
        connectionState = 'connected';
        lastError = '';
        const contentType = cleanText(res.headers.get('content-type') || '');
        if (contentType.includes('application/json')) return await res.json();
        return { text: await res.text() };
      } catch (err) {
        lastErr = err;
        if (attempt < MAX_CONNECT_ATTEMPTS) {
          connectionState = 'reconnecting';
          await sleep(Math.min(1500, RETRY_BASE_DELAY_MS * attempt));
          continue;
        }
      }
    }
    connectionState = 'disconnected';
    const errText = cleanText(lastErr && lastErr.message ? lastErr.message : '');
    throw new Error(errText || 'Unable to connect after 5 reconnect attempts.');
  }

  function normalizeAgent(row) {
    const id = cleanText(row && (row.id || row.agent_id || '')).trim();
    if (!id) return null;
    return {
      id,
      name: cleanText(row && row.name ? row.name : id),
      state: cleanText(row && row.state ? row.state : 'Idle'),
      model: cleanText(
        row && (row.runtime_model || row.model_name || row.model || row.model_override || '')
          ? (row.runtime_model || row.model_name || row.model || row.model_override)
          : ''
      ),
    };
  }

  function normalizeMessage(row, fallbackId) {
    const roleRaw = cleanText(row && row.role ? row.role : 'agent').toLowerCase();
    const role = roleRaw === 'assistant' ? 'agent' : roleRaw;
    const text = cleanText(
      row && (row.text || row.content || row.message || row.response || row.output || '')
        ? (row.text || row.content || row.message || row.response || row.output)
        : ''
    );
    if (!text.trim()) return null;
    return {
      id: cleanText(row && row.id ? row.id : `${fallbackId}-${Math.random().toString(36).slice(2, 10)}`),
      role: role === 'user' ? 'user' : 'agent',
      text,
      ts: Number(row && row.ts ? row.ts : Date.now()),
    };
  }

  async function loadAgents() {
    let rows = [];
    try {
      rows = await apiRequest('/api/agents?view=sidebar&authority=runtime');
      if (!Array.isArray(rows)) rows = [];
    } catch (_) {
      const fallback = await apiRequest('/api/agents');
      rows = Array.isArray(fallback) ? fallback : [];
    }
    agents = rows.map(normalizeAgent).filter(Boolean);
    if (!selectedAgentId || !agents.some((row) => row.id === selectedAgentId)) {
      selectedAgentId = agents.length ? agents[0].id : '';
      if (selectedAgentId) await loadSession(selectedAgentId);
      else messages = [];
    }
  }

  async function createAgent() {
    if (creatingAgent) return;
    creatingAgent = true;
    try {
      const ordinal = agents.length + 1;
      const created = await apiRequest('/api/agents', {
        method: 'POST',
        body: {
          name: `Agent ${ordinal}`,
          role: 'assistant',
          provider: 'auto',
          model: 'auto',
        },
      });
      const createdId = cleanText(created && created.agent_id ? created.agent_id : '').trim();
      await loadAgents();
      if (createdId) {
        selectedAgentId = createdId;
        await loadSession(createdId);
      }
    } catch (err) {
      lastError = cleanText(err && err.message ? err.message : 'agent_create_failed');
    } finally {
      creatingAgent = false;
    }
  }

  async function loadSession(agentId) {
    const id = cleanText(agentId || '').trim();
    if (!id) {
      messages = [];
      return;
    }
    selectedAgentId = id;
    const data = await apiRequest(`/api/agents/${encodeURIComponent(id)}/session`);
    const rows = Array.isArray(data && data.messages) ? data.messages : [];
    messages = rows
      .map((row, idx) => normalizeMessage(row, `${id}-${idx}`))
      .filter(Boolean);
  }

  async function sendMessage() {
    const id = cleanText(selectedAgentId || '').trim();
    const text = composer.trim();
    if (!id || !text || sending) return;
    composer = '';
    sending = true;
    messages = messages.concat([
      { id: `local-user-${Date.now()}`, role: 'user', text, ts: Date.now() },
    ]);
    try {
      const res = await apiRequest(`/api/agents/${encodeURIComponent(id)}/message`, {
        method: 'POST',
        body: { message: text },
      });
      const answer = cleanText(res && (res.response || res.content || '')).trim();
      messages = messages.concat([
        {
          id: `local-agent-${Date.now()}`,
          role: 'agent',
          text: answer || 'Agent returned an empty response.',
          ts: Date.now(),
        },
      ]);
      await loadAgents();
    } catch (err) {
      const errorText = cleanText(err && err.message ? err.message : 'message_send_failed');
      messages = messages.concat([
        { id: `local-error-${Date.now()}`, role: 'agent', text: `Error: ${errorText}`, ts: Date.now() },
      ]);
      lastError = errorText;
    } finally {
      sending = false;
    }
  }

  async function loadChannels() {
    channelsLoading = true;
    try {
      const data = await apiRequest('/api/channels');
      channels = Array.isArray(data && data.channels) ? data.channels : [];
    } catch (err) {
      lastError = cleanText(err && err.message ? err.message : 'channels_load_failed');
      channels = [];
    } finally {
      channelsLoading = false;
    }
  }

  async function loadSkills() {
    skillsLoading = true;
    try {
      const [installedData, browseData] = await Promise.all([
        apiRequest('/api/skills'),
        apiRequest(`/api/clawhub/browse?sort=trending&limit=40`),
      ]);
      skills = Array.isArray(installedData && installedData.skills) ? installedData.skills : [];
      marketplace = Array.isArray(browseData && browseData.items) ? browseData.items : [];
    } catch (err) {
      lastError = cleanText(err && err.message ? err.message : 'skills_load_failed');
      skills = [];
      marketplace = [];
    } finally {
      skillsLoading = false;
    }
  }

  async function installSkill(slug) {
    const value = cleanText(slug).trim();
    if (!value || installingSlug) return;
    installingSlug = value;
    try {
      await apiRequest('/api/clawhub/install', { method: 'POST', body: { slug: value } });
      await loadSkills();
    } catch (err) {
      lastError = cleanText(err && err.message ? err.message : 'skill_install_failed');
    } finally {
      installingSlug = '';
    }
  }

  async function refreshAll() {
    refreshing = true;
    try {
      await Promise.all([loadAgents(), loadChannels(), loadSkills()]);
    } finally {
      refreshing = false;
    }
  }

  $: selectedAgent = agents.find((row) => row.id === selectedAgentId) || null;
  $: filteredMarketplace = marketplace.filter((row) => {
    const q = cleanText(skillsQuery).trim().toLowerCase();
    if (!q) return true;
    const tags = Array.isArray(row && row.tags) ? row.tags.join(' ') : '';
    const haystack = `${cleanText(row && row.slug)} ${cleanText(row && row.name)} ${cleanText(row && row.description)} ${tags}`.toLowerCase();
    return haystack.includes(q);
  });

  onMount(() => {
    let cancelled = false;
    const boot = async () => {
      loading = true;
      try {
        await refreshAll();
      } finally {
        if (!cancelled) loading = false;
      }
    };
    boot();
    const agentsTicker = setInterval(() => {
      if (document.visibilityState === 'visible') loadAgents();
    }, 8000);
    const channelsTicker = setInterval(() => {
      if (document.visibilityState === 'visible' && page === 'channels') loadChannels();
    }, 20000);
    return () => {
      cancelled = true;
      clearInterval(agentsTicker);
      clearInterval(channelsTicker);
    };
  });
</script>

<main class="shell">
  <aside class="sidebar">
    <div class="brand">INFRING</div>
    <button class="new-agent" on:click={createAgent} disabled={creatingAgent}>
      {creatingAgent ? 'Creating...' : 'New Agent'}
    </button>
    <nav class="nav">
      <button class:active={page === 'chat'} on:click={() => (page = 'chat')}>Chat</button>
      <button class:active={page === 'channels'} on:click={() => (page = 'channels')}>Channels</button>
      <button class:active={page === 'skills'} on:click={() => (page = 'skills')}>Plugins</button>
    </nav>
    <div class="agent-list">
      {#each agents as agent}
        <button class:active={selectedAgentId === agent.id} on:click={() => loadSession(agent.id)}>
          <div class="agent-name">{agent.name}</div>
          <div class="agent-meta">{agent.state}{agent.model ? ` - ${agent.model}` : ''}</div>
        </button>
      {/each}
      {#if !agents.length}
        <div class="empty">No agents running.</div>
      {/if}
    </div>
  </aside>

  <section class="content">
    <header class="topbar">
      <div class="title">{page === 'chat' ? (selectedAgent ? selectedAgent.name : 'Chat') : page === 'channels' ? 'Channels' : 'Plugin Marketplace'}</div>
      <div class="status">
        <span class={`dot ${connectionState}`}></span>
        <span>{connectionState}</span>
      </div>
      <button class="refresh" on:click={refreshAll} disabled={refreshing}>{refreshing ? 'Refreshing...' : 'Refresh'}</button>
    </header>

    {#if loading}
      <div class="panel loading">Booting dashboard...</div>
    {:else if page === 'chat'}
      <div class="chat-layout">
        <div class="messages">
          {#each messages as msg}
            <div class={`message ${msg.role}`}>
              <div class="bubble">{msg.text}</div>
            </div>
          {/each}
          {#if !messages.length}
            <div class="empty">Select an agent and start chatting.</div>
          {/if}
        </div>
        <div class="composer">
          <input bind:value={composer} placeholder={selectedAgentId ? `Message ${selectedAgent ? selectedAgent.name : 'agent'}...` : 'Create or select an agent first'} on:keydown={(e) => e.key === 'Enter' && sendMessage()} disabled={!selectedAgentId || sending} />
          <button on:click={sendMessage} disabled={!selectedAgentId || sending || !composer.trim()}>{sending ? 'Sending...' : 'Send'}</button>
        </div>
      </div>
    {:else if page === 'channels'}
      <div class="panel">
        {#if channelsLoading}
          <div class="loading">Loading channels...</div>
        {:else}
          <div class="grid">
            {#each channels as ch}
              <article>
                <h3>{ch.display_name || ch.name}</h3>
                <p>{ch.description || ''}</p>
                <div class="meta">{ch.category || 'general'} - {ch.setup_type || 'form'}</div>
                <div class={`badge ${ch.connected ? 'ok' : ch.configured ? 'warn' : 'off'}`}>
                  {ch.connected ? 'Connected' : ch.configured ? 'Configured' : 'Not Configured'}
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <div class="panel">
        <div class="skills-head">
          <input bind:value={skillsQuery} placeholder="Search plugins..." />
        </div>
        {#if skillsLoading}
          <div class="loading">Loading plugins...</div>
        {:else}
          <h3>Installed ({skills.length})</h3>
          <div class="chips">
            {#each skills as skill}
              <span class="chip">{skill.name}</span>
            {/each}
            {#if !skills.length}
              <span class="chip muted">No plugins installed</span>
            {/if}
          </div>
          <h3>Marketplace</h3>
          <div class="grid">
            {#each filteredMarketplace as item}
              <article>
                <h3>{item.title || item.name || item.slug}</h3>
                <p>{item.description || ''}</p>
                <div class="meta">{item.author || 'Unknown'} - {item.runtime || 'prompt_only'}</div>
                <button on:click={() => installSkill(item.slug)} disabled={installingSlug === item.slug}>
                  {installingSlug === item.slug ? 'Installing...' : 'Install'}
                </button>
              </article>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    {#if lastError}
      <footer class="error">{lastError}</footer>
    {/if}
  </section>
</main>

<style>
  :global(body) { margin: 0; background: #0b1017; color: #edf2f7; font-family: "IBM Plex Sans", "Segoe UI", sans-serif; }
  .shell { min-height: 100vh; display: grid; grid-template-columns: 280px 1fr; }
  .sidebar { border-right: 1px solid #1f2a37; background: linear-gradient(180deg, #0f1724, #0a1018); padding: 16px; display: flex; flex-direction: column; gap: 12px; }
  .brand { font-weight: 700; letter-spacing: 0.16em; font-size: 14px; color: #66a4ff; }
  .new-agent { border: 1px solid #2b3f56; border-radius: 10px; padding: 10px; background: #112137; color: #dce9ff; cursor: pointer; }
  .nav { display: grid; grid-template-columns: 1fr; gap: 6px; }
  .nav button { border: 1px solid #213145; border-radius: 8px; padding: 8px; text-align: left; background: #0f1a28; color: #b7c7dd; cursor: pointer; }
  .nav button.active { border-color: #3f82ff; color: #f3f8ff; background: #13253d; }
  .agent-list { flex: 1; overflow: auto; display: flex; flex-direction: column; gap: 6px; }
  .agent-list button { border: 1px solid #1f2f44; border-radius: 8px; background: #0f1824; color: #d7e5f8; padding: 8px; text-align: left; cursor: pointer; }
  .agent-list button.active { border-color: #3f82ff; box-shadow: 0 0 0 1px rgba(63, 130, 255, 0.25) inset; }
  .agent-name { font-weight: 600; font-size: 13px; }
  .agent-meta { opacity: 0.75; font-size: 12px; margin-top: 2px; }
  .content { display: flex; flex-direction: column; min-height: 100vh; }
  .topbar { height: 56px; border-bottom: 1px solid #1f2a37; display: flex; align-items: center; gap: 16px; padding: 0 16px; background: #0f1724; }
  .title { font-size: 15px; font-weight: 600; flex: 1; }
  .status { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; text-transform: capitalize; }
  .dot { width: 8px; height: 8px; border-radius: 99px; background: #f59e0b; }
  .dot.connected { background: #22c55e; }
  .dot.reconnecting { background: #f59e0b; }
  .dot.disconnected { background: #ef4444; }
  .refresh { border: 1px solid #2b3f56; border-radius: 8px; background: #13253d; color: #dce9ff; padding: 8px 10px; cursor: pointer; }
  .panel { padding: 16px; overflow: auto; flex: 1; }
  .chat-layout { display: grid; grid-template-rows: 1fr auto; height: calc(100vh - 56px); }
  .messages { overflow: auto; padding: 16px; display: flex; flex-direction: column; gap: 10px; }
  .message { display: flex; }
  .message.user { justify-content: flex-end; }
  .bubble { max-width: 78%; border: 1px solid #213145; border-radius: 12px; padding: 10px 12px; background: #121d2a; white-space: pre-wrap; }
  .message.user .bubble { background: #1f3553; border-color: #35567d; }
  .composer { border-top: 1px solid #1f2a37; padding: 12px; display: grid; grid-template-columns: 1fr auto; gap: 8px; background: #0f1724; }
  .composer input { border: 1px solid #223247; border-radius: 10px; background: #0b1320; color: #edf2f7; padding: 10px 12px; }
  .composer button { border: 1px solid #3f82ff; border-radius: 10px; background: #2a6ee8; color: #fff; padding: 10px 14px; cursor: pointer; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 12px; }
  article { border: 1px solid #213145; border-radius: 12px; padding: 12px; background: #101926; display: flex; flex-direction: column; gap: 8px; }
  article h3 { margin: 0; font-size: 14px; }
  article p { margin: 0; opacity: 0.86; font-size: 13px; line-height: 1.35; }
  article button { border: 1px solid #3f82ff; border-radius: 8px; background: #275fca; color: white; padding: 7px 10px; cursor: pointer; }
  .meta { font-size: 12px; opacity: 0.7; }
  .badge { font-size: 11px; border-radius: 999px; padding: 4px 8px; width: fit-content; border: 1px solid transparent; }
  .badge.ok { background: rgba(34, 197, 94, 0.15); border-color: rgba(34, 197, 94, 0.35); }
  .badge.warn { background: rgba(245, 158, 11, 0.13); border-color: rgba(245, 158, 11, 0.35); }
  .badge.off { background: rgba(148, 163, 184, 0.13); border-color: rgba(148, 163, 184, 0.28); }
  .skills-head { margin-bottom: 12px; }
  .skills-head input { width: 100%; box-sizing: border-box; border: 1px solid #223247; border-radius: 10px; background: #0b1320; color: #edf2f7; padding: 10px 12px; }
  .chips { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 14px; }
  .chip { border: 1px solid #2a3f58; border-radius: 999px; padding: 6px 10px; font-size: 12px; background: #111d2c; }
  .chip.muted { opacity: 0.7; }
  .empty { opacity: 0.72; padding: 16px 8px; font-size: 13px; }
  .loading { opacity: 0.82; padding: 24px 12px; }
  .error { margin: 0; border-top: 1px solid #50222b; color: #fecaca; background: #2d1016; padding: 8px 12px; font-size: 12px; }

  @media (max-width: 900px) {
    .shell { grid-template-columns: 1fr; }
    .sidebar { border-right: 0; border-bottom: 1px solid #1f2a37; }
    .chat-layout { height: auto; min-height: calc(100vh - 220px); }
  }
</style>
