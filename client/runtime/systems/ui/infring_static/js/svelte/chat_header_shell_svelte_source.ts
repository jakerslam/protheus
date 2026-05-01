const COMPONENT_TAG = 'infring-chat-header-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-chat-header-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let uiTick = 0;
  let unsubscribe = null;
  let timer = 0;

  function bridge() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.current === 'function') {
      var current = storeBridge.current();
      if (current) return current;
    }
    if (storeBridge && typeof storeBridge.root === 'function') return storeBridge.root();
    return typeof window !== 'undefined' && window.InfringApp ? window.InfringApp : null;
  }

  function chatPage() {
    return typeof window !== 'undefined' && window.InfringChatPage ? window.InfringChatPage : null;
  }

  function call(owner, name) {
    var args = Array.prototype.slice.call(arguments, 2);
    if (owner && typeof owner[name] === 'function') {
      try { return owner[name].apply(owner, args); } catch (_) { return undefined; }
    }
    return undefined;
  }

  function callPageOrApp(name) {
    var args = Array.prototype.slice.call(arguments, 1);
    var page = chatPage();
    var result = call.apply(null, [page, name].concat(args));
    if (typeof result !== 'undefined') return result;
    var app = appStore();
    return call.apply(null, [app, name].concat(args));
  }

  function bump() {
    uiTick += 1;
  }

  function currentAgent(_tick) {
    var page = chatPage();
    if (page && page.currentAgent) return page.currentAgent;
    var app = appStore();
    return app && app.currentAgent ? app.currentAgent : null;
  }

  function archived(agentRef, _tick) {
    var page = chatPage();
    if (page && typeof page.isCurrentAgentArchived === 'function') {
      return !!call(page, 'isCurrentAgentArchived');
    }
    var app = appStore();
    if (app && typeof app.isArchivedLikeAgent === 'function') return !!call(app, 'isArchivedLikeAgent', agentRef);
    return !!(agentRef && (agentRef.archived || String(agentRef.state || '').toLowerCase().indexOf('archived') >= 0));
  }

  function displayEmoji(agentRef, _tick) {
    return String(callPageOrApp('displayAgentEmoji', agentRef) || '');
  }

  function statusState(agentRef, _tick) {
    return String(callPageOrApp('agentStatusState', agentRef) || 'unknown');
  }

  function statusLabel(agentRef, _tick) {
    return String(callPageOrApp('agentStatusLabel', agentRef) || 'Unknown');
  }

  function heartStates(agentRef, _tick) {
    var states = callPageOrApp('agentHeartStates', agentRef);
    return Array.isArray(states) ? states : [];
  }

  function heartShowsInfinity(agentRef, _tick) {
    return !!callPageOrApp('agentHeartShowsInfinity', agentRef);
  }

  function heartLabel(agentRef, _tick) {
    return String(callPageOrApp('agentHeartMeterLabel', agentRef) || '');
  }

  function agentName(agentRef) {
    return String((agentRef && (agentRef.name || agentRef.id)) || 'Agent');
  }

  function avatarAlt(agentRef) {
    return agentName(agentRef) + ' avatar';
  }

  function toggleDrawer(event) {
    var agentRef = currentAgent(uiTick);
    if (!agentRef || archived(agentRef, uiTick)) return;
    if (event && typeof event.preventDefault === 'function') event.preventDefault();
    callPageOrApp('toggleAgentDrawer');
    bump();
  }

  function handleKeydown(event) {
    if (!event) return;
    if (event.key === 'Enter' || event.key === ' ') toggleDrawer(event);
  }

  onMount(function() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(bump);
    }
    timer = window.setInterval(bump, 250);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (timer) window.clearInterval(timer);
  });
</script>

{#if currentAgent(uiTick)}
  <div class="chat-thread-topline">
    <div class="chat-thread-profile-center">
      <div
        class="chat-thread-profile"
        class:chat-thread-profile-disabled={archived(currentAgent(uiTick), uiTick)}
        role="button"
        tabindex={archived(currentAgent(uiTick), uiTick) ? -1 : 0}
        title={archived(currentAgent(uiTick), uiTick) ? 'Archived agents are read-only until revived' : 'Agent details'}
        on:click={toggleDrawer}
        on:keydown={handleKeydown}
      >
        <div
          class="chat-thread-profile-avatar"
          class:chat-thread-profile-avatar-archived-mask={archived(currentAgent(uiTick), uiTick)}
          style:display={archived(currentAgent(uiTick), uiTick) ? 'none' : ''}
        >
          {#if currentAgent(uiTick).avatar_url}
            <img src={currentAgent(uiTick).avatar_url} alt={avatarAlt(currentAgent(uiTick))} loading="lazy" />
          {:else if displayEmoji(currentAgent(uiTick), uiTick)}
            <span>{displayEmoji(currentAgent(uiTick), uiTick)}</span>
          {:else}
            <span class="infring-logo infring-logo--agent-default" aria-hidden="true"><span class="infring-logo-glyph" aria-hidden="true">&infin;</span></span>
          {/if}
        </div>
        <div class="chat-thread-profile-info-pill">
          <div class="chat-thread-profile-meta">
            <span
              class={'agent-status-dot chat-title-status-dot status-' + statusState(currentAgent(uiTick), uiTick)}
              title={'Agent status: ' + statusLabel(currentAgent(uiTick), uiTick)}
              aria-hidden="true"
            ></span>
            <div class="chat-thread-profile-name">{agentName(currentAgent(uiTick))}</div>
          </div>
          {#if currentAgent(uiTick) && !currentAgent(uiTick).is_system_thread}
            <div class="chat-thread-heart-meter" title={heartLabel(currentAgent(uiTick), uiTick)}>
              {#each heartStates(currentAgent(uiTick), uiTick) as filled, idx ('chat-heart-' + String(currentAgent(uiTick).id || 'agent') + '-' + idx)}
                <span class="chat-thread-heart" class:is-empty={!filled} aria-hidden="true">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M12 21s-7-4.2-9-8.4C1.5 9.5 3.3 6 6.4 6c2.2 0 3.4 1.2 3.9 2.1.5-.9 1.7-2.1 3.9-2.1 3.1 0 4.9 3.5 3.4 6.6-2 4.2-9 8.4-9 8.4z"></path>
                  </svg>
                </span>
              {/each}
              {#if heartShowsInfinity(currentAgent(uiTick), uiTick)}
                <span class="chat-thread-heart-infinity" aria-hidden="true">&infin;</span>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
