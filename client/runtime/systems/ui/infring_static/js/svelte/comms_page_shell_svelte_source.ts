const COMPONENT_TAG = 'infring-comms-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-comms-page-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'comms';
  export let panelRole = 'page';
  export let routeContract = 'comms';
  export let parentOwnedData = false;

  const MAX_EVENTS = 200;
  const SSE_RETRY_MS = 2000;

  let topology = { nodes: [], edges: [], connected: true };
  let events = [];
  let loading = true;
  let loadError = '';
  let sseSource = null;
  let sseRetryTimer = null;
  let topologyRefreshTimer = null;
  let eventKeyIndex = Object.create(null);
  let showSendModal = false;
  let showTaskModal = false;
  let sendFrom = '';
  let sendTo = '';
  let sendMsg = '';
  let sendLoading = false;
  let taskTitle = '';
  let taskDesc = '';
  let taskAssign = '';
  let taskLoading = false;

  $: rootNodeRows = rootNodes();

  function api() {
    return typeof window !== 'undefined' ? window.InfringAPI : null;
  }

  function toast() {
    return typeof window !== 'undefined' ? window.InfringToast : null;
  }

  function notifySuccess(message) {
    var t = toast();
    if (t && typeof t.success === 'function') t.success(message);
  }

  function notifyError(message) {
    var t = toast();
    if (t && typeof t.error === 'function') t.error(message);
  }

  function timeAgo(value) {
    if (!value) return '';
    var parsed = Date.parse(String(value));
    if (!Number.isFinite(parsed)) return '';
    var secs = Math.floor((Date.now() - parsed) / 1000);
    if (!Number.isFinite(secs) || secs < 0) secs = 0;
    if (secs < 60) return secs + 's ago';
    if (secs < 3600) return Math.floor(secs / 60) + 'm ago';
    if (secs < 86400) return Math.floor(secs / 3600) + 'h ago';
    return Math.floor(secs / 86400) + 'd ago';
  }

  function normalizeTopologyPayload(payload) {
    var source = payload && typeof payload === 'object' ? payload : {};
    var nested = source.topology && typeof source.topology === 'object' ? source.topology : source;
    return {
      nodes: Array.isArray(nested.nodes) ? nested.nodes.slice() : [],
      edges: Array.isArray(nested.edges) ? nested.edges.slice() : [],
      connected: nested.connected !== false
    };
  }

  function normalizeEventsPayload(payload) {
    if (Array.isArray(payload)) return payload.slice();
    var source = payload && typeof payload === 'object' ? payload : {};
    return Array.isArray(source.events) ? source.events.slice() : [];
  }

  function normalizeEvent(payload) {
    var source = payload && typeof payload === 'object' ? payload : {};
    var fromId = String(source.from_agent_id || source.from || '');
    var toId = String(source.to_agent_id || source.to || '');
    var detail = String(source.detail || source.message || source.title || '');
    return {
      id: String(source.id || source.event_id || ''),
      kind: String(source.kind || 'unknown'),
      from_agent_id: fromId,
      to_agent_id: toId,
      source_name: String(source.source_name || source.from_name || source.from_agent_name || fromId),
      target_name: String(source.target_name || source.to_name || source.to_agent_name || toId),
      detail: detail,
      message: String(source.message || detail),
      title: String(source.title || ''),
      timestamp: String(source.timestamp || source.ts || ''),
      _key: ''
    };
  }

  function eventKey(event) {
    if (!event || typeof event !== 'object') return 'event:invalid';
    if (event.id) return 'id:' + event.id;
    return 'fallback:' + String(event.timestamp || '') + '|' + String(event.kind || '') + '|' + String(event.from_agent_id || '') + '|' + String(event.to_agent_id || '');
  }

  function replaceEvents(rows) {
    eventKeyIndex = Object.create(null);
    var next = [];
    (Array.isArray(rows) ? rows : []).forEach(function(row) {
      var event = normalizeEvent(row);
      var key = eventKey(event);
      if (eventKeyIndex[key]) return;
      eventKeyIndex[key] = true;
      event._key = key;
      next.push(event);
    });
    events = next.sort(function(a, b) {
      return String(b.timestamp || '').localeCompare(String(a.timestamp || ''));
    }).slice(0, MAX_EVENTS);
  }

  function pushEvent(rawEvent) {
    var event = normalizeEvent(rawEvent);
    var key = eventKey(event);
    if (eventKeyIndex[key]) return false;
    eventKeyIndex[key] = true;
    event._key = key;
    events = [event].concat(events).slice(0, MAX_EVENTS);
    return true;
  }

  function isFairyNode(node) {
    var row = node && typeof node === 'object' ? node : {};
    var probe = [
      String(row.id || ''),
      String(row.name || ''),
      String(row.role || ''),
      String(row.archetype || '')
    ].join(' ').toLowerCase();
    return probe.indexOf('fairy') >= 0 || probe.indexOf('faerie') >= 0;
  }

  function rootNodes() {
    var childIds = {};
    (topology.edges || []).forEach(function(edge) {
      if (edge && edge.kind === 'parent_child') childIds[edge.to] = true;
    });
    return (topology.nodes || []).filter(function(node) { return node && !childIds[node.id]; });
  }

  function childrenOf(id) {
    var childIds = {};
    (topology.edges || []).forEach(function(edge) {
      if (edge && edge.kind === 'parent_child' && edge.from === id) childIds[edge.to] = true;
    });
    return (topology.nodes || []).filter(function(node) { return node && childIds[node.id]; });
  }

  function peersOf(id) {
    var peerIds = {};
    (topology.edges || []).forEach(function(edge) {
      if (!edge || edge.kind !== 'peer') return;
      if (edge.from === id) peerIds[edge.to] = true;
      if (edge.to === id) peerIds[edge.from] = true;
    });
    return (topology.nodes || []).filter(function(node) { return node && peerIds[node.id]; });
  }

  function stateBadgeClass(state) {
    switch (state) {
      case 'Running': return 'badge badge-success';
      case 'Suspended': return 'badge badge-warning';
      case 'Terminated':
      case 'Crashed': return 'badge badge-danger';
      default: return 'badge badge-dim';
    }
  }

  function eventBadgeClass(kind) {
    switch (kind) {
      case 'agent_message': return 'badge badge-info';
      case 'agent_spawned': return 'badge badge-success';
      case 'agent_terminated': return 'badge badge-danger';
      case 'task_posted': return 'badge badge-warning';
      case 'task_claimed': return 'badge badge-info';
      case 'task_completed': return 'badge badge-success';
      default: return 'badge badge-dim';
    }
  }

  function eventLabel(kind) {
    switch (kind) {
      case 'agent_message': return 'Message';
      case 'agent_spawned': return 'Spawned';
      case 'agent_terminated': return 'Terminated';
      case 'task_posted': return 'Task Posted';
      case 'task_claimed': return 'Task Claimed';
      case 'task_completed': return 'Task Done';
      default: return String(kind || 'unknown');
    }
  }

  async function loadData() {
    loading = true;
    loadError = '';
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') throw new Error('Shell API client is unavailable.');
      var results = await Promise.all([
        client.get('/api/comms/topology'),
        client.get('/api/comms/events?limit=200')
      ]);
      topology = normalizeTopologyPayload(results[0]);
      replaceEvents(normalizeEventsPayload(results[1]));
      startSSE();
    } catch (e) {
      loadError = e && e.message ? e.message : 'Could not load comms data.';
    }
    loading = false;
  }

  function startSSE() {
    stopSSE();
    var client = api();
    if (!client || typeof window === 'undefined' || typeof window.EventSource !== 'function') return;
    var url = String(client.baseUrl || '') + '/api/comms/events/stream';
    if (client.apiKey) url += '?token=' + encodeURIComponent(client.apiKey);
    sseSource = new EventSource(url);
    sseSource.onmessage = function(ev) {
      if (ev.data === 'ping') return;
      try {
        var event = JSON.parse(ev.data);
        pushEvent(event);
        if (event.kind === 'agent_spawned' || event.kind === 'agent_terminated') scheduleTopologyRefresh();
      } catch (_) {}
    };
    sseSource.onerror = function() {
      stopSSE();
      sseRetryTimer = setTimeout(startSSE, SSE_RETRY_MS);
    };
  }

  function stopSSE() {
    if (sseSource) {
      sseSource.close();
      sseSource = null;
    }
    if (sseRetryTimer) {
      clearTimeout(sseRetryTimer);
      sseRetryTimer = null;
    }
  }

  function scheduleTopologyRefresh() {
    if (topologyRefreshTimer) return;
    topologyRefreshTimer = setTimeout(function() {
      topologyRefreshTimer = null;
      refreshTopology();
    }, 200);
  }

  async function refreshTopology() {
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') return;
      topology = normalizeTopologyPayload(await client.get('/api/comms/topology'));
    } catch (_) {}
  }

  function openSendModal() {
    sendFrom = '';
    sendTo = '';
    sendMsg = '';
    showSendModal = true;
  }

  async function submitSend() {
    if (!sendFrom || !sendTo || !String(sendMsg || '').trim()) return;
    sendLoading = true;
    try {
      var client = api();
      if (!client || typeof client.post !== 'function') throw new Error('Shell API client is unavailable.');
      await client.post('/api/comms/send', {
        from_agent_id: sendFrom,
        to_agent_id: sendTo,
        message: sendMsg
      });
      notifySuccess('Message sent');
      showSendModal = false;
    } catch (e) {
      notifyError(e && e.message ? e.message : 'Send failed');
    }
    sendLoading = false;
  }

  function openTaskModal() {
    taskTitle = '';
    taskDesc = '';
    taskAssign = '';
    showTaskModal = true;
  }

  async function submitTask() {
    if (!String(taskTitle || '').trim()) return;
    taskLoading = true;
    try {
      var client = api();
      if (!client || typeof client.post !== 'function') throw new Error('Shell API client is unavailable.');
      var body = { title: taskTitle, description: taskDesc };
      if (taskAssign) body.assigned_to = taskAssign;
      await client.post('/api/comms/task', body);
      notifySuccess('Task posted');
      showTaskModal = false;
    } catch (e) {
      notifyError(e && e.message ? e.message : 'Task failed');
    }
    taskLoading = false;
  }

  onMount(loadData);
  onDestroy(function() {
    stopSSE();
    if (topologyRefreshTimer) clearTimeout(topologyRefreshTimer);
  });
</script>

<div class="page-header">
  <h2>Agent Comms</h2>
  <div class="flex items-center gap-2">
    <button class="btn btn-primary btn-sm" on:click={openSendModal}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 2L11 13"/><path d="M22 2l-7 20-4-9-9-4z"/></svg>
      Send Message
    </button>
    <button class="btn btn-ghost btn-sm" on:click={openTaskModal}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20M17 5H9.5a3.5 3.5 0 000 7h5a3.5 3.5 0 010 7H6"/></svg>
      Post Task
    </button>
    <button class="btn btn-ghost btn-sm" on:click={loadData} title="Refresh">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0115-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 01-15 6.7L3 16"/></svg>
    </button>
  </div>
</div>

<div class="page-body">
  {#if loading}
    <div style="animation:fadeIn 0.2s">
      <div class="card mb-4"><div class="skeleton skeleton-text" style="width:160px;margin-bottom:8px"></div><div class="skeleton skeleton-card" style="height:120px"></div></div>
      <div class="card"><div class="skeleton skeleton-text" style="width:120px;margin-bottom:8px"></div><div class="skeleton skeleton-card" style="height:200px"></div></div>
    </div>
  {:else if loadError}
    <div class="error-state" style="animation:fadeIn 0.3s">
      <h3 style="color:var(--error)">Connection Error</h3>
      <p class="text-xs text-dim">{loadError}</p>
      <button class="btn btn-primary btn-sm" on:click={loadData} style="margin-top:8px">Retry</button>
    </div>
  {:else}
    <div style="animation:fadeIn 0.3s">
      <div class="card mb-4">
        <div class="card-header">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline;margin-right:4px;vertical-align:-2px"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><path d="M8.59 13.51l6.83 3.98M15.41 6.51l-6.82 3.98"/></svg>
          Agent Topology
          <span class="badge badge-dim" style="margin-left:8px;font-weight:400">{topology.nodes.length} agents</span>
        </div>
        <div style="padding:8px 0;font-family:var(--font-mono);font-size:12px;line-height:1.8">
          {#if topology.nodes.length === 0}
            <div class="text-dim" style="text-align:center;padding:24px">No agents running</div>
          {/if}
          {#each rootNodeRows as root (root.id)}
            <div class="comms-topo-tree">
              <div class="comms-topo-node" title={root.id}>
                <span class={stateBadgeClass(root.state)} style="font-size:10px;padding:1px 6px">{root.state}</span>
                <strong style="margin:0 4px">{root.name}</strong>
                {#if isFairyNode(root)}<span class="badge badge-info" style="font-size:9px;padding:1px 6px;margin-right:4px">FAIRY</span>{/if}
                <span class="text-dim">{root.model}</span>
                {#each peersOf(root.id) as peer (peer.id)}
                  <span class="text-dim" style="margin-left:8px">&harr; {peer.name}</span>
                {/each}
              </div>
              {#each childrenOf(root.id) as child, ci (child.id)}
                <div class="comms-topo-child">
                  <span class="comms-topo-branch">{ci < childrenOf(root.id).length - 1 ? '|-- ' : '\\-- '}</span>
                  <span class={stateBadgeClass(child.state)} style="font-size:10px;padding:1px 6px">{child.state}</span>
                  <strong style="margin:0 4px">{child.name}</strong>
                  {#if isFairyNode(child)}<span class="badge badge-info" style="font-size:9px;padding:1px 6px;margin-right:4px">FAIRY</span>{/if}
                  <span class="text-dim">{child.model}</span>
                </div>
              {/each}
            </div>
          {/each}
        </div>
      </div>

      <div class="card">
        <div class="card-header flex justify-between items-center">
          <div>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline;margin-right:4px;vertical-align:-2px"><path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z"/></svg>
            Live Event Feed
          </div>
          <div class="flex items-center gap-2">
            <span class="badge badge-success" style="font-size:9px;padding:2px 6px;animation:pulse-ring 2s infinite">LIVE</span>
            <span class="text-xs text-dim">{events.length} events</span>
          </div>
        </div>
        <div style="max-height:400px;overflow-y:auto">
          {#if events.length === 0}
            <div class="text-dim" style="text-align:center;padding:24px">No inter-agent events yet</div>
          {/if}
          {#each events as ev (ev._key)}
            <div class="comms-event-row">
              <span class="comms-event-time text-xs text-dim">{timeAgo(ev.timestamp)}</span>
              <span class={eventBadgeClass(ev.kind)} style="font-size:10px;padding:1px 6px;min-width:70px;text-align:center">{eventLabel(ev.kind)}</span>
              <span style="font-weight:600;font-size:12px">{ev.source_name}</span>
              {#if ev.target_name}<span class="text-dim">-&gt; {ev.target_name}</span>{/if}
              <span class="comms-event-detail text-dim text-xs" style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{ev.detail}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  {#if showSendModal}
    <div style="position:fixed;inset:0;z-index:9999;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,0.5);backdrop-filter:blur(4px)" on:click={function() { showSendModal = false; }}>
      <div class="card" style="width:420px;max-width:90vw" on:click|stopPropagation>
        <div class="card-header">Send Agent Message</div>
        <div style="display:flex;flex-direction:column;gap:12px;margin-top:12px">
          <label class="text-xs text-dim">From Agent</label>
          <select bind:value={sendFrom} class="input" style="width:100%">
            <option value="">Select agent...</option>
            {#each topology.nodes as node (node.id)}
              <option value={node.id}>{node.name} ({node.state})</option>
            {/each}
          </select>
          <label class="text-xs text-dim">To Agent</label>
          <select bind:value={sendTo} class="input" style="width:100%">
            <option value="">Select agent...</option>
            {#each topology.nodes as node (node.id)}
              <option value={node.id}>{node.name} ({node.state})</option>
            {/each}
          </select>
          <label class="text-xs text-dim">Message</label>
          <textarea bind:value={sendMsg} class="input" rows="3" placeholder="Type a message..." style="width:100%;resize:vertical"></textarea>
          <div class="flex gap-2" style="justify-content:flex-end">
            <button class="btn btn-ghost btn-sm" on:click={function() { showSendModal = false; }}>Cancel</button>
            <button class="btn btn-primary btn-sm" on:click={submitSend} disabled={sendLoading || !sendFrom || !sendTo || !String(sendMsg || '').trim()}>{sendLoading ? 'Sending...' : 'Send'}</button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if showTaskModal}
    <div style="position:fixed;inset:0;z-index:9999;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,0.5);backdrop-filter:blur(4px)" on:click={function() { showTaskModal = false; }}>
      <div class="card" style="width:420px;max-width:90vw" on:click|stopPropagation>
        <div class="card-header">Post Task</div>
        <div style="display:flex;flex-direction:column;gap:12px;margin-top:12px">
          <label class="text-xs text-dim">Title</label>
          <input type="text" bind:value={taskTitle} class="input" placeholder="Task title..." style="width:100%">
          <label class="text-xs text-dim">Description</label>
          <textarea bind:value={taskDesc} class="input" rows="3" placeholder="Task description..." style="width:100%;resize:vertical"></textarea>
          <label class="text-xs text-dim">Assign To (optional)</label>
          <select bind:value={taskAssign} class="input" style="width:100%">
            <option value="">Unassigned</option>
            {#each topology.nodes as node (node.id)}
              <option value={node.id}>{node.name}</option>
            {/each}
          </select>
          <div class="flex gap-2" style="justify-content:flex-end">
            <button class="btn btn-ghost btn-sm" on:click={function() { showTaskModal = false; }}>Cancel</button>
            <button class="btn btn-primary btn-sm" on:click={submitTask} disabled={taskLoading || !String(taskTitle || '').trim()}>{taskLoading ? 'Posting...' : 'Post Task'}</button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
