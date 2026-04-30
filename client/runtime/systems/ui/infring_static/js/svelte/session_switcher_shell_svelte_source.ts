const COMPONENT_TAG = 'infring-session-switcher-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-session-switcher-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  let sessions = [];
  let open = false;
  let unsubs = [];

  function chatPage() {
    return (typeof window !== 'undefined' && window.InfringChatPage) || null;
  }

  function chatStore() {
    return (typeof window !== 'undefined' && window.InfringChatStore) || null;
  }

  function syncFromPage() {
    var page = chatPage();
    sessions = page && Array.isArray(page.sessions) ? page.sessions : [];
  }

  function callPage(fn) {
    var page = chatPage();
    if (!page || typeof page[fn] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    return page[fn].apply(page, args);
  }

  function sessionId(row) {
    return String((row && (row.session_id || row.id || row.key)) || '');
  }

  function sessionLabel(row) {
    var explicit = String((row && (row._label || row.label || row.name)) || '').trim();
    if (explicit) return explicit;
    var id = sessionId(row);
    return id ? ('Session ' + id.substring(0, 8)) : 'Session';
  }

  function messageCount(row) {
    var value = Number(row && row.message_count);
    return Number.isFinite(value) ? value : 0;
  }

  function toggleOpen(event) {
    if (event && typeof event.stopPropagation === 'function') event.stopPropagation();
    syncFromPage();
    open = !open;
  }

  function closeOpen() {
    open = false;
  }

  function createSession(event) {
    if (event && typeof event.stopPropagation === 'function') event.stopPropagation();
    callPage('createSession');
  }

  function switchSession(row, event) {
    if (event && typeof event.stopPropagation === 'function') event.stopPropagation();
    if (!row || row.active) return;
    var id = sessionId(row);
    if (!id) return;
    callPage('switchSession', id);
    open = false;
  }

  onMount(function() {
    syncFromPage();
    var store = chatStore();
    if (store && store.sessions && typeof store.sessions.subscribe === 'function') {
      unsubs.push(store.sessions.subscribe(function(rows) {
        sessions = Array.isArray(rows) ? rows : [];
      }));
    }
    if (typeof window !== 'undefined') {
      window.addEventListener('click', closeOpen);
    }
  });

  onDestroy(function() {
    for (var i = 0; i < unsubs.length; i += 1) {
      if (typeof unsubs[i] === 'function') unsubs[i]();
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('click', closeOpen);
    }
  });
</script>

<div style="position:relative" on:click|stopPropagation>
  <button class="btn btn-ghost btn-sm" type="button" on:click={toggleOpen} title="Sessions" style="position:relative">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/></svg>
    {#if sessions.length > 1}
      <span class="session-count-badge">{sessions.length}</span>
    {/if}
  </button>
  {#if open}
    <infring-taskbar-menu-shell class="session-dropdown dashboard-dropdown-surface">
      <div class="session-dropdown-header dashboard-dropdown-header">
        <span class="text-xs font-bold">Sessions</span>
        <button class="btn btn-ghost btn-sm" type="button" on:click={createSession} style="padding:2px 6px;font-size:11px">+ New</button>
      </div>
      {#each sessions as session (sessionId(session))}
        <div class={"session-item" + (session && session.active ? ' active' : '')} on:click={(event) => switchSession(session, event)}>
          <span class={"session-dot" + (session && session.active ? ' active' : '')}></span>
          <div style="flex:1;min-width:0">
            <div class="text-xs font-bold truncate">{sessionLabel(session)}</div>
            <div class="text-xs text-dim">{messageCount(session)} messages</div>
          </div>
        </div>
      {/each}
      {#if !sessions.length}
        <div class="text-xs text-dim" style="padding:8px 12px;text-align:center">No sessions</div>
      {/if}
    </infring-taskbar-menu-shell>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
