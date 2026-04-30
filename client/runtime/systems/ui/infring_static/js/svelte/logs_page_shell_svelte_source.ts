const COMPONENT_TAG = 'infring-logs-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-logs-page-shell', shadow: 'none' }} />
<script>
  import { tick, onMount, onDestroy } from 'svelte';

  let tab = 'live';
  let entries = [];
  let levelFilter = '';
  let textFilter = '';
  let autoRefresh = true;
  let hovering = false;
  let loading = true;
  let loadError = '';
  let eventSource = null;
  let pollTimer = null;
  let streamConnected = false;
  let streamConnecting = true;
  let streamPaused = false;
  let entryKeyIndex = Object.create(null);
  let auditEntries = [];
  let tipHash = '';
  let chainValid = null;
  let filterAction = '';
  let auditLoading = false;
  let auditLoadError = '';
  let agents = [];
  let unsubscribeStore = null;
  let logContainer;

  $: filteredEntries = entries.filter(function(entry) {
    var needle = String(textFilter || '').toLowerCase();
    if (levelFilter && classifyLevel(entry.action) !== levelFilter) return false;
    if (!needle) return true;
    return (String(entry.action || '') + ' ' + String(entry.detail || '') + ' ' + String(entry.agent_id || '')).toLowerCase().indexOf(needle) !== -1;
  });
  $: filteredAuditEntries = filterAction ? auditEntries.filter(function(entry) { return entry.action === filterAction; }) : auditEntries;
  $: connectionLabel = streamPaused ? 'Paused' : (streamConnecting ? 'Connecting...' : (streamConnected ? 'Live' : (pollTimer ? 'Polling' : 'Disconnected')));
  $: connectionClass = streamPaused ? 'paused' : (streamConnecting ? 'connecting' : (streamConnected ? 'live' : (pollTimer ? 'polling' : 'disconnected')));

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
    agents = snapshot && Array.isArray(snapshot.agents) ? snapshot.agents.slice() : [];
  }

  function safeTimestamp(value) {
    var raw = String(value || '').trim();
    var parsed = raw ? Date.parse(raw) : NaN;
    return Number.isFinite(parsed) ? new Date(parsed).toISOString() : '';
  }

  function normalizeEntry(raw) {
    var row = raw && typeof raw === 'object' ? raw : {};
    var seq = Number(row.seq);
    var entry = {
      seq: Number.isFinite(seq) ? seq : null,
      timestamp: safeTimestamp(row.timestamp) || new Date().toISOString(),
      action: String(row.action || ''),
      detail: String(row.detail || ''),
      agent_id: String(row.agent_id || ''),
      outcome: String(row.outcome || ''),
      payload: row.payload
    };
    entry._key = entryKey(entry);
    return entry;
  }

  function entryKey(entry) {
    if (entry && entry.seq !== null) return 'seq:' + String(entry.seq);
    return 'fallback:' + String(entry && entry.timestamp || '') + '|' + String(entry && entry.action || '') + '|' + String(entry && entry.detail || '');
  }

  function resetEntryIndex() {
    entryKeyIndex = Object.create(null);
  }

  function ingestEntry(raw, skipScroll) {
    var entry = normalizeEntry(raw);
    var key = entry._key || entryKey(entry);
    if (entryKeyIndex[key]) return false;
    entryKeyIndex[key] = true;
    entries = entries.concat([entry]);
    if (entries.length > 500) {
      var removed = entries.slice(0, entries.length - 500);
      entries = entries.slice(entries.length - 500);
      removed.forEach(function(row) { delete entryKeyIndex[row._key || entryKey(row)]; });
    }
    if (!skipScroll && autoRefresh && !hovering) scrollToBottom();
    return true;
  }

  function ingestEntries(rows) {
    resetEntryIndex();
    entries = [];
    (Array.isArray(rows) ? rows : []).forEach(function(row) { ingestEntry(row, true); });
    entries = entries.slice().sort(function(a, b) {
      if (a.seq !== null && b.seq !== null) return a.seq - b.seq;
      return String(a.timestamp || '').localeCompare(String(b.timestamp || ''));
    }).slice(-500);
  }

  async function scrollToBottom() {
    await tick();
    if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
  }

  function classifyLevel(action) {
    var value = String(action || '').toLowerCase();
    if (value.indexOf('error') !== -1 || value.indexOf('fail') !== -1 || value.indexOf('crash') !== -1) return 'error';
    if (value.indexOf('warn') !== -1 || value.indexOf('deny') !== -1 || value.indexOf('block') !== -1) return 'warn';
    return 'info';
  }

  async function fetchLogs() {
    if (loading) loadError = '';
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await client.get('/api/audit/recent?n=200');
      ingestEntries(data && data.entries);
      if (autoRefresh && !hovering) scrollToBottom();
      loading = false;
    } catch(e) {
      if (loading) {
        loadError = e && e.message ? e.message : 'Could not load logs.';
        loading = false;
      }
    }
  }

  function startPolling() {
    streamConnected = false;
    streamConnecting = false;
    fetchLogs();
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = setInterval(function() {
      if (autoRefresh && !hovering && tab === 'live' && !streamPaused) fetchLogs();
    }, 2000);
  }

  function startStreaming() {
    stopStreaming();
    streamConnecting = true;
    var client = api();
    var url = '/api/logs/stream';
    var token = client && typeof client.getToken === 'function' ? client.getToken() : '';
    if (token) url += '?token=' + encodeURIComponent(token);
    try {
      eventSource = new EventSource(url);
    } catch(_) {
      startPolling();
      return;
    }
    eventSource.onopen = function() {
      streamConnected = true;
      streamConnecting = false;
      loading = false;
      loadError = '';
    };
    eventSource.onmessage = function(event) {
      if (streamPaused) return;
      try { ingestEntry(JSON.parse(event.data), false); } catch(_) {}
    };
    eventSource.onerror = function() {
      streamConnected = false;
      streamConnecting = false;
      stopStreaming();
      startPolling();
    };
  }

  function stopStreaming() {
    if (eventSource) {
      eventSource.close();
      eventSource = null;
    }
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  function togglePause() {
    streamPaused = !streamPaused;
    if (!streamPaused && streamConnected) scrollToBottom();
  }

  function clearLogs() {
    entries = [];
    resetEntryIndex();
  }

  function exportLogs() {
    var lines = filteredEntries.map(function(entry) {
      return (safeTimestamp(entry.timestamp) || String(entry.timestamp || '')) + ' [' + entry.action + '] ' + (entry.detail || '');
    });
    var blob = new Blob([lines.join('\\n')], { type: 'text/plain' });
    var url = URL.createObjectURL(blob);
    var link = document.createElement('a');
    link.href = url;
    link.download = 'infring-logs-' + new Date().toISOString().slice(0, 10) + '.txt';
    link.click();
    URL.revokeObjectURL(url);
  }

  function switchTab(nextTab) {
    tab = nextTab;
    if (nextTab === 'audit' && !auditEntries.length && !auditLoading) loadAudit();
  }

  async function loadAudit() {
    auditLoading = true;
    auditLoadError = '';
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await client.get('/api/audit/recent?n=200');
      auditEntries = Array.isArray(data && data.entries) ? data.entries : [];
      tipHash = String(data && data.tip_hash || '');
    } catch(e) {
      auditEntries = [];
      auditLoadError = e && e.message ? e.message : 'Could not load audit log.';
    }
    auditLoading = false;
  }

  function auditAgentName(agentId) {
    if (!agentId) return '-';
    var agent = agents.find(function(row) { return row && row.id === agentId; });
    return agent ? (agent.name || agent.id) : String(agentId).substring(0, 8) + '...';
  }

  function friendlyAction(action) {
    var map = {
      AgentSpawn: 'Agent Created', AgentKill: 'Agent Stopped', AgentTerminated: 'Agent Stopped',
      ToolInvoke: 'Tool Used', ToolResult: 'Tool Completed', AgentMessage: 'Message',
      NetworkAccess: 'Network Access', ShellExec: 'Shell Command', FileAccess: 'File Access',
      MemoryAccess: 'Memory Access', AuthAttempt: 'Login Attempt', AuthSuccess: 'Login Success',
      AuthFailure: 'Login Failed', CapabilityDenied: 'Permission Denied', RateLimited: 'Rate Limited'
    };
    return map[action] || String(action || 'Unknown').replace(/([A-Z])/g, ' $1').trim();
  }

  async function verifyChain() {
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await client.get('/api/audit/verify');
      chainValid = data && data.valid === true;
      var t = toast();
      if (chainValid && t && typeof t.success === 'function') t.success('Audit chain verified - ' + (data.entries || 0) + ' entries valid');
      if (!chainValid && t && typeof t.error === 'function') t.error('Audit chain broken!');
    } catch(e) {
      chainValid = false;
      var t = toast();
      if (t && typeof t.error === 'function') t.error('Chain verification failed: ' + (e && e.message ? e.message : 'Unknown error'));
    }
  }

  onMount(function() {
    var bridge = appStore();
    if (bridge && typeof bridge.subscribe === 'function') unsubscribeStore = bridge.subscribe(syncAgents);
    else if (bridge && typeof bridge.snapshot === 'function') syncAgents(bridge.snapshot());
    startStreaming();
  });

  onDestroy(function() {
    stopStreaming();
    stopPolling();
    resetEntryIndex();
    if (typeof unsubscribeStore === 'function') unsubscribeStore();
  });
</script>

<div class="page-header page-header-subtabs-center">
  <div><div class="tabs mt-3" role="tablist"><div class="tab" role="tab" on:click={() => navigate('runtime')}>Runtime</div><div class="tab" role="tab" on:click={() => navigate('analytics')}>Analytics</div><div class="tab active" role="tab" on:click={() => navigate('logs')}>Logs</div></div></div>
  {#if tab === 'live'}
    <div class="flex gap-2 items-center">
      <span class={'live-indicator ' + connectionClass}><span class="live-dot"></span><span>{connectionLabel}</span></span>
      <select class="form-select" style="width:100px" bind:value={levelFilter}><option value="">All</option><option value="info">INFO</option><option value="warn">WARN</option><option value="error">ERROR</option></select>
      <input class="form-input" style="width:180px" placeholder="Search..." bind:value={textFilter}>
      <button class="btn btn-ghost btn-sm" on:click={togglePause}>{streamPaused ? 'Resume' : 'Pause'}</button>
      <button class="btn btn-ghost btn-sm" on:click={clearLogs}>Clear</button><button class="btn btn-ghost btn-sm" on:click={exportLogs}>Export</button>
      <div class="toggle" class:active={autoRefresh} on:click={() => autoRefresh = !autoRefresh} title="Auto-scroll"></div><span class="text-xs text-dim">{autoRefresh ? 'Auto-scroll' : 'Scroll locked'}</span>
    </div>
  {:else}
    <div class="flex gap-2 items-center"><button class="btn btn-ghost btn-sm" on:click={verifyChain}>Verify Chain</button>{#if chainValid !== null}<span class={'badge ' + (chainValid ? 'badge-running' : 'badge-crashed')}>{chainValid ? 'VALID' : 'BROKEN'}</span>{/if}</div>
  {/if}
</div>
<div class="tabs"><div class="tab" class:active={tab === 'live'} on:click={() => switchTab('live')}>Live</div><div class="tab" class:active={tab === 'audit'} on:click={() => switchTab('audit')}>Audit Trail</div></div>
<div class="page-body">
  {#if tab === 'live'}
    {#if loading}<div class="loading-state"><div class="spinner"></div><span>Connecting to log stream...</span></div>
    {:else if loadError}<div class="error-state"><span class="error-icon">!</span><p>{loadError}</p><button class="btn btn-ghost btn-sm" on:click={fetchLogs}>Retry</button></div>
    {:else}<div class="card" style="font-family:monospace;max-height:70vh;overflow-y:auto" bind:this={logContainer} on:mouseenter={() => hovering = true} on:mouseleave={() => hovering = false}>
      {#each filteredEntries as entry (entry._key)}<div class="log-entry"><span class="log-timestamp">{new Date(entry.timestamp).toLocaleTimeString()}</span><span class={'log-level log-level-' + classifyLevel(entry.action)}>{classifyLevel(entry.action).toUpperCase()}</span><span class="text-xs" style="color:var(--text-dim);margin-right:6px">[{entry.action}]</span><span class="text-xs">{entry.detail}</span></div>{/each}
      {#if !filteredEntries.length}<div class="empty-state" style="padding:20px"><h4>No log entries yet</h4><p class="hint">Activity will appear here as agents run.</p></div>{/if}
    </div>{/if}
  {:else}
    {#if auditLoading}<div class="loading-state"><div class="spinner"></div><span>Loading audit log...</span></div>
    {:else if auditLoadError}<div class="error-state"><span class="error-icon">!</span><p>{auditLoadError}</p><button class="btn btn-ghost btn-sm" on:click={loadAudit}>Retry</button></div>
    {:else}<div><div class="card mb-4" style="border-left:3px solid var(--accent)"><div class="font-bold" style="font-size:13px;margin-bottom:4px">Tamper-Evident Audit Trail</div><div class="text-sm text-dim" style="line-height:1.6">Every agent action is logged with a cryptographic hash chain. Use "Verify Chain" to confirm no entries have been altered or deleted.</div></div>
      <div class="flex gap-2 mb-4 items-center"><select class="form-select" style="width:180px" bind:value={filterAction}><option value="">All Actions</option><option value="AgentSpawn">Agent Created</option><option value="AgentKill">Agent Stopped</option><option value="AgentMessage">Message</option><option value="ToolInvoke">Tool Used</option><option value="NetworkAccess">Network Access</option><option value="ShellExec">Shell Command</option><option value="FileAccess">File Access</option><option value="MemoryAccess">Memory Access</option><option value="AuthAttempt">Login Attempt</option></select><span class="text-sm text-dim">{filteredAuditEntries.length} of {auditEntries.length} entries</span>{#if tipHash}<span class="text-xs text-dim">tip: {tipHash.substring(0, 16)}...</span>{/if}</div>
      {#if filteredAuditEntries.length}<div class="table-wrap"><table><thead><tr><th>#</th><th>Timestamp</th><th>Agent</th><th>Action</th><th>Detail</th><th>Outcome</th></tr></thead><tbody>{#each filteredAuditEntries as entry (entry.seq || entry.timestamp)}<tr><td>{entry.seq}</td><td class="text-xs" style="white-space:nowrap">{new Date(entry.timestamp).toLocaleString()}</td><td class="truncate" style="max-width:120px" title={entry.agent_id}>{auditAgentName(entry.agent_id)}</td><td><span class="badge badge-created">{friendlyAction(entry.action)}</span></td><td class="truncate" style="max-width:200px" title={entry.detail}>{entry.detail}</td><td>{entry.outcome}</td></tr>{/each}</tbody></table></div>{:else}<div class="empty-state"><h4>No audit entries yet</h4><p class="hint">Activity will appear here as agents operate.</p></div>{/if}
    </div>{/if}
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
