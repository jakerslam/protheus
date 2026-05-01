const COMPONENT_TAG = 'infring-eyes-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-eyes-page-shell', shadow: 'none' }} />
<script>
  import { onMount } from 'svelte';

  const allowedStatus = { active: true, paused: true, dormant: true };

  let eyes = [];
  let loading = true;
  let loadError = '';
  let saving = false;
  let formError = '';
  let form = {
    name: '',
    status: 'active',
    url: '',
    apiKey: '',
    cadenceHours: 4,
    topics: ''
  };

  function navigate(page) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(page);
    else if (typeof window !== 'undefined') window.location.hash = page;
  }

  function numberValue(value, fallback) {
    var parsed = Number(value);
    if (!Number.isFinite(parsed)) return Number(fallback || 0);
    return parsed;
  }

  function normalizeStatus(value) {
    var lowered = String(value || 'active').trim().toLowerCase();
    return allowedStatus[lowered] ? lowered : 'active';
  }

  function normalizeTopics(raw) {
    var source = String(raw || '');
    var out = [];
    var seen = Object.create(null);
    source.split(/[,\n]/).forEach(function(piece) {
      var topic = String(piece || '').trim();
      if (!topic) return;
      var key = topic.toLowerCase();
      if (seen[key]) return;
      seen[key] = true;
      out.push(topic);
    });
    return out;
  }

  function safeHost(urlValue) {
    try { return new URL(String(urlValue || '')).host || ''; } catch (_) { return ''; }
  }

  function normalizeEyeRow(row) {
    var source = row && typeof row === 'object' ? row : {};
    var endpointUrl = String(source.endpoint_url || source.url || '').trim();
    return {
      id: String(source.id || ''),
      name: String(source.name || '').trim(),
      status: normalizeStatus(source.status),
      cadence_hours: Math.max(1, Math.min(168, Math.round(numberValue(source.cadence_hours, 4)))),
      endpoint_url: endpointUrl,
      endpoint_host: String(source.endpoint_host || safeHost(endpointUrl) || '').trim(),
      source: String(source.source || '').trim() || 'system',
      api_key_present: !!source.api_key_present,
      topics: Array.isArray(source.topics) ? source.topics : normalizeTopics(source.topics),
      updated_at: String(source.updated_at || source.updated_ts || source.ts || '').trim()
    };
  }

  function normalizeFormPayload() {
    var name = String(form.name || '').trim();
    var url = String(form.url || '').trim();
    var apiKey = String(form.apiKey || '').trim();
    var topics = normalizeTopics(form.topics);
    var cadence = Math.max(1, Math.min(168, Math.round(numberValue(form.cadenceHours, 4))));
    return {
      name: name || (url ? safeHost(url) : 'eye'),
      status: normalizeStatus(form.status),
      url: url,
      api_key: apiKey,
      cadence_hours: cadence,
      topics: topics
    };
  }

  function statusBadgeClass(status) {
    if (status === 'active') return 'badge-success';
    if (status === 'paused') return 'badge-warn';
    if (status === 'dormant') return 'badge-muted';
    return 'badge-dim';
  }

  function sourceLabel(eye) {
    if (!eye || typeof eye !== 'object') return 'system';
    if (eye.endpoint_host) return eye.endpoint_host;
    if (eye.endpoint_url) return safeHost(eye.endpoint_url) || eye.endpoint_url;
    if (eye.api_key_present) return 'api-key';
    return eye.source || 'system';
  }

  function formatUpdated(ts) {
    var raw = String(ts || '').trim();
    if (!raw) return '-';
    var date = new Date(raw);
    if (Number.isNaN(date.getTime())) return '-';
    return date.toLocaleString([], {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    });
  }

  function resetForm() {
    form = {
      name: '',
      status: 'active',
      url: '',
      apiKey: '',
      cadenceHours: 4,
      topics: ''
    };
    formError = '';
  }

  async function loadEyes() {
    loading = true;
    loadError = '';
    try {
      var api = typeof window !== 'undefined' ? window.InfringAPI : null;
      if (!api || typeof api.get !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await api.get('/api/eyes');
      eyes = (Array.isArray(data && data.eyes) ? data.eyes : []).map(normalizeEyeRow);
    } catch (e) {
      eyes = [];
      loadError = e && e.message ? e.message : 'Could not load eyes.';
    }
    loading = false;
  }

  async function addEye() {
    formError = '';
    var payload = normalizeFormPayload();
    if (!String(payload.url || '').trim() && !String(payload.api_key || '').trim()) {
      formError = 'Provide a source URL, an API key, or both.';
      return;
    }
    saving = true;
    try {
      var api = typeof window !== 'undefined' ? window.InfringAPI : null;
      if (!api || typeof api.post !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await api.post('/api/eyes', payload);
      if (!data || data.ok === false) throw new Error((data && data.error) ? String(data.error) : 'Eyes update failed');
      var normalizedEye = normalizeEyeRow(data && data.eye ? data.eye : payload);
      var addedName = normalizedEye.name || payload.name || 'eye';
      if (window.InfringToast && typeof window.InfringToast.success === 'function') {
        window.InfringToast.success((data.created ? 'Added ' : 'Updated ') + '"' + addedName + '"');
      }
      form = Object.assign({}, form, {
        apiKey: '',
        url: '',
        topics: (payload.topics || []).join(', '),
        name: data.created ? '' : form.name
      });
      await loadEyes();
    } catch (e) {
      formError = e && e.message ? e.message : 'Could not save eye.';
      if (window.InfringToast && typeof window.InfringToast.error === 'function') window.InfringToast.error(formError);
    }
    saving = false;
  }

  onMount(loadEyes);
</script>

<div>
  <div class="page-header page-header-subtabs-center">
    <div class="tabs mt-3" role="tablist">
      <div class="tab" role="tab" on:click={() => navigate('skills')}>Apps</div>
      <div class="tab" role="tab" on:click={() => navigate('channels')}>Channels</div>
      <div class="tab active" role="tab" on:click={() => navigate('eyes')}>Eyes</div>
      <div class="tab" role="tab" on:click={() => navigate('hands')}>Hands</div>
    </div>
  </div>
  <div class="page-body">
    {#if loading}
      <div class="loading-state"><div class="spinner"></div><span>Loading eyes...</span></div>
    {:else if loadError}
      <div class="error-state">
        <span class="error-icon">!</span>
        <p>{loadError}</p>
        <button class="btn btn-ghost btn-sm" type="button" on:click={loadEyes}>Retry</button>
      </div>
    {:else}
      <div class="card" style="margin-bottom:16px">
        <div class="card-header">Add Eye</div>
        <p class="text-sm text-dim" style="margin:0 0 12px">
          Syncs directly with the system eyes catalog. Add a URL source, an API key, or both.
        </p>
        <div class="grid grid-cols-2" style="gap:10px">
          <div class="form-group" style="margin:0">
            <label>Name</label>
            <input class="form-input" type="text" bind:value={form.name} placeholder="My Eye">
          </div>
          <div class="form-group" style="margin:0">
            <label>Status</label>
            <select class="form-select" bind:value={form.status}>
              <option value="active">active</option>
              <option value="paused">paused</option>
              <option value="dormant">dormant</option>
              <option value="disabled">disabled</option>
            </select>
          </div>
          <div class="form-group" style="margin:0;grid-column:1 / span 2">
            <label>Source URL</label>
            <input class="form-input" type="text" bind:value={form.url} placeholder="https://example.com/feed">
          </div>
          <div class="form-group" style="margin:0;grid-column:1 / span 2">
            <label>API Key</label>
            <input class="form-input" type="password" bind:value={form.apiKey} placeholder="Optional API key">
          </div>
          <div class="form-group" style="margin:0">
            <label>Cadence (hours)</label>
            <input class="form-input" type="number" min="1" max="168" bind:value={form.cadenceHours}>
          </div>
          <div class="form-group" style="margin:0">
            <label>Topics</label>
            <input class="form-input" type="text" bind:value={form.topics} placeholder="ai, agents, infra">
          </div>
        </div>
        {#if formError}
          <div class="text-xs" style="color:var(--danger,#ef4444);margin-top:8px">{formError}</div>
        {/if}
        <div class="text-xs text-dim" style="margin-top:8px">
          API keys are hashed before being stored in the local catalog.
        </div>
        <div class="flex gap-2 mt-3">
          <button class="btn btn-primary btn-sm" type="button" disabled={saving} on:click={addEye}>{saving ? 'Saving...' : 'Add Eye'}</button>
          <button class="btn btn-ghost btn-sm" type="button" disabled={saving} on:click={resetForm}>Reset</button>
          <button class="btn btn-ghost btn-sm" type="button" disabled={saving} on:click={loadEyes}>Refresh</button>
        </div>
      </div>

      <div class="card">
        <div class="card-header">System Eyes</div>
        {#if eyes.length}
          <table class="table" style="margin-top:8px">
            <thead>
              <tr>
                <th>Name</th>
                <th>Status</th>
                <th>Source</th>
                <th>Cadence</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {#each eyes as eye (eye.id)}
                <tr>
                  <td>
                    <div class="font-bold">{eye.name}</div>
                    <div class="text-xs text-dim">{eye.id}</div>
                  </td>
                  <td><span class={'badge ' + statusBadgeClass(eye.status)}>{eye.status}</span></td>
                  <td>{sourceLabel(eye)}</td>
                  <td>{eye.cadence_hours}h</td>
                  <td>{formatUpdated(eye.updated_at)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <infring-chat-stream-shell class="empty-state">
            <h4>No eyes configured</h4>
            <p class="hint">Add your first eye using URL or API key above.</p>
          </infring-chat-stream-shell>
        {/if}
      </div>
    {/if}
  </div>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
