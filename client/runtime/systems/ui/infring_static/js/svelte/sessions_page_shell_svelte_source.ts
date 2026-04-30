const COMPONENT_TAG = 'infring-sessions-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sessions-page-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  let tab = 'sessions';
  let sessions = [];
  let agents = [];
  let searchFilter = '';
  let loading = true;
  let loadError = '';
  let memAgentId = '';
  let kvPairs = [];
  let showAdd = false;
  let newKey = '';
  let newValue = '""';
  let editingKey = null;
  let editingValue = '';
  let memLoading = false;
  let memLoadError = '';
  let unsubscribeStore = null;

  $: agentMap = agents.reduce(function(map, agent) {
    if (agent && agent.id) map[agent.id] = agent.name || agent.id;
    return map;
  }, {});
  $: enrichedSessions = sessions.map(function(session) {
    return Object.assign({}, session, { agent_name: agentMap[session.agent_id] || session.agent_name || '' });
  });
  $: filteredSessions = enrichedSessions.filter(function(session) {
    var needle = String(searchFilter || '').toLowerCase();
    if (!needle) return true;
    return String(session.agent_name || '').toLowerCase().indexOf(needle) !== -1 ||
      String(session.agent_id || '').toLowerCase().indexOf(needle) !== -1;
  });

  function api() {
    return typeof window !== 'undefined' ? window.InfringAPI : null;
  }

  function toast() {
    return typeof window !== 'undefined' ? window.InfringToast : null;
  }

  function appStore() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function navigate(page) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(page);
    else if (typeof window !== 'undefined') window.location.hash = page;
  }

  function syncAgents(snapshot) {
    var source = snapshot && Array.isArray(snapshot.agents) ? snapshot.agents : [];
    agents = source.slice();
  }

  function notifyError(message) {
    var t = toast();
    if (t && typeof t.error === 'function') t.error(message);
  }

  function notifySuccess(message) {
    var t = toast();
    if (t && typeof t.success === 'function') t.success(message);
  }

  function normalizeKey(raw) {
    var key = String(raw || '').trim().replace(/[\u0000-\u001F\u007F]/g, '');
    return key.length > 256 ? key.slice(0, 256).trim() : key;
  }

  function parseValue(rawText) {
    try { return JSON.parse(rawText); } catch (_) { return rawText; }
  }

  function stringifyValue(rawValue) {
    if (rawValue && typeof rawValue === 'object') {
      try { return JSON.stringify(rawValue, null, 2); } catch (_) {}
    }
    return String(rawValue);
  }

  function shortSessionId(value) {
    var id = String(value || '');
    return id ? id.substring(0, 8) + '...' : '-';
  }

  function createdLabel(value) {
    if (!value) return '-';
    try { return new Date(value).toLocaleString(); } catch (_) { return '-'; }
  }

  async function loadSessions() {
    loading = true;
    loadError = '';
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await client.get('/api/sessions');
      sessions = Array.isArray(data && data.sessions) ? data.sessions : [];
    } catch (e) {
      sessions = [];
      loadError = e && e.message ? e.message : 'Could not load sessions.';
    }
    loading = false;
  }

  async function loadKv() {
    if (!memAgentId) {
      kvPairs = [];
      return;
    }
    memLoading = true;
    memLoadError = '';
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await client.get('/api/memory/agents/' + encodeURIComponent(memAgentId) + '/kv');
      kvPairs = Array.isArray(data && data.kv_pairs) ? data.kv_pairs : [];
    } catch(e) {
      kvPairs = [];
      memLoadError = e && e.message ? e.message : 'Could not load memory data.';
    }
    memLoading = false;
  }

  async function addKey() {
    var key = normalizeKey(newKey);
    if (!memAgentId || !key) {
      notifyError('Memory key is required');
      return;
    }
    try {
      var client = api();
      if (!client || typeof client.put !== 'function') throw new Error('Shell API client is unavailable.');
      await client.put('/api/memory/agents/' + encodeURIComponent(memAgentId) + '/kv/' + encodeURIComponent(key), { value: parseValue(newValue) });
      showAdd = false;
      newKey = '';
      newValue = '""';
      notifySuccess('Key "' + key + '" saved');
      await loadKv();
    } catch(e) {
      notifyError('Failed to save key: ' + (e && e.message ? e.message : 'Unknown error'));
    }
  }

  function startEdit(kv) {
    editingKey = kv && kv.key ? kv.key : null;
    editingValue = stringifyValue(kv && Object.prototype.hasOwnProperty.call(kv, 'value') ? kv.value : '');
  }

  function cancelEdit() {
    editingKey = null;
    editingValue = '';
  }

  async function saveEdit() {
    var key = normalizeKey(editingKey);
    if (!memAgentId || !key) {
      notifyError('Memory key is invalid');
      return;
    }
    try {
      var client = api();
      if (!client || typeof client.put !== 'function') throw new Error('Shell API client is unavailable.');
      await client.put('/api/memory/agents/' + encodeURIComponent(memAgentId) + '/kv/' + encodeURIComponent(key), { value: parseValue(editingValue) });
      editingKey = null;
      editingValue = '';
      notifySuccess('Key "' + key + '" updated');
      await loadKv();
    } catch(e) {
      notifyError('Failed to save: ' + (e && e.message ? e.message : 'Unknown error'));
    }
  }

  function deleteKey(key) {
    var t = toast();
    var run = async function() {
      try {
        var client = api();
        if (!client || typeof client.del !== 'function') throw new Error('Shell API client is unavailable.');
        await client.del('/api/memory/agents/' + encodeURIComponent(memAgentId) + '/kv/' + encodeURIComponent(key));
        notifySuccess('Key "' + key + '" deleted');
        await loadKv();
      } catch(e) {
        notifyError('Failed to delete key: ' + (e && e.message ? e.message : 'Unknown error'));
      }
    };
    if (t && typeof t.confirm === 'function') t.confirm('Delete Key', 'Delete key "' + key + '"? This cannot be undone.', run);
    else run();
  }

  function deleteSession(sessionId) {
    var t = toast();
    var run = async function() {
      try {
        var client = api();
        if (!client || typeof client.del !== 'function') throw new Error('Shell API client is unavailable.');
        await client.del('/api/sessions/' + encodeURIComponent(sessionId));
        sessions = sessions.filter(function(row) { return row.session_id !== sessionId; });
        notifySuccess('Session deleted');
      } catch(e) {
        notifyError('Failed to delete session: ' + (e && e.message ? e.message : 'Unknown error'));
      }
    };
    if (t && typeof t.confirm === 'function') t.confirm('Delete Session', 'This will permanently remove the session and its messages.', run);
    else run();
  }

  function openInChat(session) {
    var bridge = appStore();
    var match = agents.find(function(agent) { return agent && agent.id === session.agent_id; });
    if (bridge && match && typeof bridge.set === 'function') bridge.set('pendingAgent', match);
    navigate('agents');
  }

  function handleEscape(event) {
    if (showAdd && event && event.key === 'Escape') showAdd = false;
  }

  onMount(function() {
    var bridge = appStore();
    if (bridge && typeof bridge.subscribe === 'function') unsubscribeStore = bridge.subscribe(syncAgents);
    else if (bridge && typeof bridge.snapshot === 'function') syncAgents(bridge.snapshot());
    loadSessions();
  });

  onDestroy(function() {
    if (typeof unsubscribeStore === 'function') unsubscribeStore();
  });
</script>

<svelte:window on:keydown={handleEscape} />

<div class="page-header manage-sessions-header">
  <div class="manage-sessions-header-left">
    <div class="tabs mt-3" role="tablist">
      <div class="tab" role="tab" class:active={false} on:click={() => navigate('agents')}>Agents</div>
      <div class="tab active" role="tab" on:click={() => navigate('sessions')}>Sessions</div>
      <div class="tab" role="tab" class:active={false} on:click={() => navigate('approvals')}>Approvals</div>
    </div>
  </div>
  {#if tab === 'sessions'}
    <div class="manage-sessions-header-filter">
      <input class="form-input" placeholder="Filter by agent..." bind:value={searchFilter}>
    </div>
  {/if}
</div>
<div class="tabs">
  <div class="tab" class:active={tab === 'sessions'} on:click={() => tab = 'sessions'}>Sessions</div>
  <div class="tab" class:active={tab === 'memory'} on:click={() => tab = 'memory'}>Memory</div>
</div>
<div class="page-body">
  {#if tab === 'sessions'}
    <div>
      {#if loading}
        <div class="loading-state"><div class="spinner"></div><span>Loading sessions...</span></div>
      {:else if loadError}
        <div class="error-state">
          <span class="error-icon">!</span><p>{loadError}</p>
          <button class="btn btn-ghost btn-sm" on:click={loadSessions}>Retry</button>
        </div>
      {:else}
        <div class="card mb-4" style="border-left:3px solid var(--accent)">
          <div class="font-bold" style="font-size:13px;margin-bottom:4px">Conversation Sessions</div>
          <div class="text-sm text-dim" style="line-height:1.6">Each conversation with an agent creates a session. Sessions store the full message history so you can resume conversations later, or review past interactions.</div>
        </div>
        {#if filteredSessions.length}
          <div class="table-wrap">
            <table>
              <thead><tr><th>Session</th><th>Agent</th><th>Messages</th><th>Created</th><th>Actions</th></tr></thead>
              <tbody>
                {#each filteredSessions as session (session.session_id)}
                  <tr>
                    <td class="text-xs truncate" style="font-family:monospace;max-width:120px" title={session.session_id || ''}>{shortSessionId(session.session_id)}</td>
                    <td class="font-bold">{session.agent_name || session.agent_id || ''}</td>
                    <td>{session.message_count == null ? '' : session.message_count}</td>
                    <td class="text-xs">{createdLabel(session.created_at)}</td>
                    <td><button class="btn btn-primary btn-sm" on:click={() => openInChat(session)}>Chat</button><button class="btn btn-danger btn-sm" on:click={() => deleteSession(session.session_id)}>Delete</button></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else if searchFilter}
          <div class="empty-state"><p>No sessions match your filter.</p></div>
        {:else}
          <div class="empty-state">
            <div class="empty-state-icon"><svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 2-10 5 10 5 10-5z"/><path d="m2 17 10 5 10-5"/><path d="m2 12 10 5 10-5"/></svg></div>
            <h3>No sessions yet</h3><p>Sessions are created when you chat with agents. Start a conversation to see session history here.</p>
            <button class="btn btn-primary" on:click={() => navigate('agents')}>Start Chatting</button>
          </div>
        {/if}
      {/if}
    </div>
  {:else}
    <div>
      <div class="flex justify-between items-center mb-4">
        <div class="info-card" style="flex:1;margin-bottom:0"><h4>Agent Memory</h4><p>Each agent has its own key-value memory store. Agents use memory to persist preferences, notes, and context between conversations.</p></div>
        <select class="form-select" style="width:200px;margin-left:16px" bind:value={memAgentId} on:change={loadKv}>
          <option value="">Select agent...</option>
          {#each agents as agent (agent.id)}<option value={agent.id}>{agent.name || agent.id}</option>{/each}
        </select>
      </div>
      {#if memLoading}
        <div class="loading-state"><div class="spinner"></div><span>Loading memory...</span></div>
      {:else if memLoadError}
        <div class="error-state"><span class="error-icon">!</span><p>{memLoadError}</p><button class="btn btn-ghost btn-sm" on:click={loadKv}>Retry</button></div>
      {:else if memAgentId}
        <div class="flex justify-between items-center mb-4"><span class="text-sm text-dim">{kvPairs.length} key(s)</span><button class="btn btn-primary btn-sm" on:click={() => showAdd = true}>+ Add Key</button></div>
        {#if kvPairs.length}
          <div class="table-wrap"><table><thead><tr><th>Key</th><th>Value</th><th style="width:140px">Actions</th></tr></thead><tbody>
            {#each kvPairs as kv (kv.key)}
              <tr>
                <td class="font-bold" style="white-space:nowrap">{kv.key}</td>
                <td>{#if editingKey !== kv.key}<pre style="font-size:11px;max-width:400px;overflow:auto;white-space:pre-wrap;margin:0;color:var(--text-dim)">{stringifyValue(kv.value)}</pre>{:else}<div><textarea class="form-textarea" bind:value={editingValue} style="font-size:11px;min-height:60px;font-family:var(--font-mono)"></textarea><div class="flex gap-2 mt-2"><button class="btn btn-primary btn-sm" on:click={saveEdit}>Save</button><button class="btn btn-ghost btn-sm" on:click={cancelEdit}>Cancel</button></div></div>{/if}</td>
                <td><div class="flex gap-2">{#if editingKey !== kv.key}<button class="btn btn-ghost btn-sm" on:click={() => startEdit(kv)}>Edit</button><button class="btn btn-danger btn-sm" on:click={() => deleteKey(kv.key)}>Delete</button>{/if}</div></td>
              </tr>
            {/each}
          </tbody></table></div>
        {:else}
          <div class="empty-state"><h4>No keys stored</h4><p class="hint">This agent has no memory entries yet. Agents create memory entries automatically during conversations, or you can add them manually.</p><button class="btn btn-primary mt-4" on:click={() => showAdd = true}>+ Add First Key</button></div>
        {/if}
      {:else}
        <div class="empty-state"><h4>Select an Agent</h4><p class="hint">Choose an agent from the dropdown above to browse and edit its memory store.</p></div>
      {/if}
    </div>
  {/if}
</div>
{#if showAdd}
  <div class="modal-overlay" on:click|self={() => showAdd = false}>
    <div class="modal">
      <div class="modal-header"><h3>Add Key</h3><button class="modal-close" on:click={() => showAdd = false}>&times;</button></div>
      <div class="form-group"><label>Key</label><input class="form-input" bind:value={newKey} placeholder="my_key"></div>
      <div class="form-group"><label>Value (JSON)</label><textarea class="form-textarea" bind:value={newValue} placeholder={'"hello"'}></textarea></div>
      <button class="btn btn-primary btn-block" on:click={addKey}>Save</button>
    </div>
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
