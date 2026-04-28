const COMPONENT_TAG = 'infring-taskbar-system-items-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-taskbar-system-items-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-system-items';
  export let parentOwnedMechanics = true;

  let uiTick = 0;
  let rootNode = null;
  let tickTimer = 0;

  function appStoreService() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }
  function app() {
    try {
      var service = appStoreService();
      return service && typeof service.current === 'function' ? service.current() : null;
    } catch (_e) {
      return null;
    }
  }
  function call(fn) {
    var s = app();
    if (!s || typeof s[fn] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return s[fn].apply(s, args); } catch (_e) { return undefined; }
  }
  function bump() {
    uiTick += 1;
  }
  function rightItems(_tick) {
    var s = app();
    var raw = s && Array.isArray(s.taskbarReorderRight) ? s.taskbarReorderRight : [];
    var normalized = call('normalizeTaskbarReorder', 'right', raw);
    return Array.isArray(normalized) && normalized.length
      ? normalized
      : ['connectivity', 'theme', 'notifications', 'search', 'auth'];
  }
  function itemStyle(item, _tick) {
    return String(call('taskbarReorderItemStyle', 'right', item) || '');
  }
  function notifications(_tick) {
    var s = app() || {};
    return Array.isArray(s.notifications) ? s.notifications : [];
  }
  function notificationBubble(_tick) {
    return (app() || {}).notificationBubble || null;
  }
  function unreadCount(_tick) {
    var value = Number((app() || {}).unreadNotifications || 0);
    return Number.isFinite(value) ? value : 0;
  }
  function themeMode(_tick) {
    return String((app() || {}).themeMode || '');
  }
  function themeResolved(_tick) {
    return String((app() || {}).theme || '');
  }
  function notificationsOpen(_tick) {
    return !!((app() || {}).notificationsOpen);
  }
  function classFromCall(fn) {
    return String(call(fn) || '');
  }
  function showUtility(title, body, event) {
    call('showTaskbarUtilityPopup', title, body, event);
  }
  function hideUtility(id) {
    call('hideDashboardPopup', id);
  }
  function reorderEvent(fn, event) {
    call(fn, 'right', event);
    bump();
  }
  function simpleEvent(fn, event) {
    call(fn, event);
    bump();
  }
  function setTheme(mode) {
    call('setTheme', mode);
    bump();
  }
  function toggleNotifications() {
    var s = app();
    if (s && typeof s.toggleNotifications === 'function') s.toggleNotifications();
    bump();
  }
  function closeNotifications() {
    var s = app();
    if (s) s.notificationsOpen = false;
    bump();
  }
  function clearNotifications() {
    var s = app();
    if (s && typeof s.clearNotifications === 'function') s.clearNotifications();
    bump();
  }
  function reopenNotification(note) {
    var s = app();
    if (s && typeof s.reopenNotification === 'function') s.reopenNotification(note);
    bump();
  }
  function dismissNotification(note, event) {
    if (event) event.stopPropagation();
    var s = app();
    if (s && typeof s.dismissNotification === 'function') s.dismissNotification(note && note.id);
    bump();
  }
  function dismissNotificationBubble(event) {
    if (event) event.stopPropagation();
    var s = app();
    if (s && typeof s.dismissNotificationBubble === 'function') s.dismissNotificationBubble();
    bump();
  }
  function formatNotificationTime(ts) {
    return String(call('formatNotificationTime', ts) || '');
  }

  onMount(function() {
    tickTimer = window.setInterval(bump, 500);
    var outside = function(event) {
      if (!notificationsOpen(uiTick)) return;
      if (rootNode && event && rootNode.contains(event.target)) return;
      closeNotifications();
    };
    document.addEventListener('pointerdown', outside, true);
    window.addEventListener('hashchange', bump, { passive: true });
    window.addEventListener('resize', bump, { passive: true });
    return function() {
      document.removeEventListener('pointerdown', outside, true);
    };
  });

  onDestroy(function() {
    if (tickTimer) window.clearInterval(tickTimer);
    window.removeEventListener('hashchange', bump);
    window.removeEventListener('resize', bump);
  });
</script>

<div class="taskbar-visual-group taskbar-visual-group-right" aria-label="System taskbar items" bind:this={rootNode} data-shell-primitive={shellPrimitive} data-wrapper-role={wrapperRole}>
  <div
    class="taskbar-reorder-box taskbar-reorder-box-right"
    on:pointerdown={(event) => reorderEvent('handleTaskbarReorderPointerDown', event)}
    on:pointerup={() => simpleEvent('cancelTaskbarDragHold')}
    on:pointercancel={() => simpleEvent('cancelTaskbarDragHold')}
    on:pointerleave={() => simpleEvent('cancelTaskbarDragHold')}
    on:dragstart={(event) => reorderEvent('handleTaskbarReorderDragStart', event)}
    on:drag={(event) => simpleEvent('handleTaskbarReorderDragMove', event)}
    on:dragenter={(event) => reorderEvent('handleTaskbarReorderDragEnter', event)}
    on:dragover={(event) => reorderEvent('handleTaskbarReorderDragOver', event)}
    on:drop={(event) => reorderEvent('handleTaskbarReorderDrop', event)}
    on:dragend={() => simpleEvent('handleTaskbarDragEnd')}
  >
    {#each rightItems(uiTick) as item (item)}
      <div class="taskbar-reorder-item" data-taskbar-item={item} style={itemStyle(item, uiTick)} draggable="true">
        {#if item === 'connectivity'}
          <div class="global-taskbar-controls">
            <button class={"health-indicator taskbar-agent-indicator " + classFromCall('runtimeFacadeClass')} type="button" on:click={() => { location.hash = 'agents'; bump(); }} title={call('runtimeFacadeTitle') || ''} aria-label="Open agents">
              <span class="taskbar-agent-indicator-icon" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg></span>
              <span class="taskbar-agent-indicator-text">{call('runtimeFacadeDisplayLabel') || ''}</span>
            </button>
          </div>
        {:else if item === 'theme'}
          <div class="theme-switcher toggle-pill" data-mode={themeMode(uiTick)} data-resolved={themeResolved(uiTick)}>
            <button class:active={themeMode(uiTick) === 'light'} class="theme-opt" on:click={() => setTheme('light')} title="Light" aria-label="Light theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"></circle><path d="M12 2v2"></path><path d="M12 20v2"></path><path d="m4.93 4.93 1.41 1.41"></path><path d="m17.66 17.66 1.41 1.41"></path><path d="M2 12h2"></path><path d="M20 12h2"></path><path d="m6.34 17.66-1.41 1.41"></path><path d="m19.07 4.93-1.41 1.41"></path></svg></button>
            <button class:active={themeMode(uiTick) === 'system'} class="theme-opt" on:click={() => setTheme('system')} title="System" aria-label="System theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="3" width="20" height="14" rx="2"></rect><path d="M8 21h8"></path><path d="M12 17v4"></path></svg></button>
            <button class:active={themeMode(uiTick) === 'dark'} class="theme-opt" on:click={() => setTheme('dark')} title="Dark" aria-label="Dark theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12.79A9 9 0 1 1 11.21 3c0 0 0 0 0 0A7 7 0 0 0 21 12.79z"></path></svg></button>
          </div>
        {:else if item === 'notifications'}
          <div id="taskbar-notification-menu-anchor" class="notif-wrap">
            <button class:notif-btn-auto-ring={!!((app() || {}).notificationBellPulse)} class="btn btn-ghost btn-sm taskbar-icon-btn notif-btn" on:click={toggleNotifications} title="Notifications" aria-label="Notifications">
              <svg class="notif-bell-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M15 17h5l-1.4-1.4A2 2 0 0 1 18 14.2V11a6 6 0 1 0-12 0v3.2a2 2 0 0 1-.6 1.4L4 17h5"></path><path d="M9 17a3 3 0 0 0 6 0"></path></svg>
              {#if unreadCount(uiTick) > 0}<span class="notif-badge">{unreadCount(uiTick) > 99 ? '99+' : unreadCount(uiTick)}</span>{/if}
            </button>
            {#if notificationsOpen(uiTick)}
              <infring-taskbar-menu-shell class="notif-dropdown dashboard-dropdown-surface" shellprimitive="taskbar-dock" wrapperrole="taskbar-menu" parentownedmechanics="true" anchorid="taskbar-notification-menu-anchor" fallbackside="bottom" layoutkey="taskbar-notification-menu">
                <div class="notif-dropdown-head dashboard-dropdown-header"><span>Notifications</span><button class="notif-clear-btn" on:click={clearNotifications}>Clear</button></div>
                {#if !notifications(uiTick).length}<div class="notif-empty">No notifications yet</div>{/if}
                {#if notifications(uiTick).length}
                  <div class="notif-list">
                    {#each notifications(uiTick) as note (note.id)}
                      <div class:unread={!note.read} class="notif-item">
                        <button class="notif-item-open" on:click={() => reopenNotification(note)}><span class={"notif-item-dot type-" + (note.type || 'info')}></span><span class="notif-item-msg">{note.message || ''}</span><span class="notif-item-time">{formatNotificationTime(note.ts)}</span></button>
                        <button class="notif-item-dismiss" on:click={(event) => dismissNotification(note, event)} title="Dismiss notification" aria-label="Dismiss notification">&times;</button>
                      </div>
                    {/each}
                  </div>
                {/if}
              </infring-taskbar-menu-shell>
            {/if}
            {#if notificationBubble(uiTick)}
              <div class="notif-bubble">
                <div class="notif-bubble-head"><span class={"notif-item-dot type-" + (notificationBubble(uiTick).type || 'info')}></span><span class="notif-bubble-time">{formatNotificationTime(notificationBubble(uiTick).ts)}</span><button class="notif-bubble-close" on:click={dismissNotificationBubble} aria-label="Dismiss notification">&times;</button></div>
                <div class="notif-bubble-msg">{notificationBubble(uiTick).message || ''}</div>
              </div>
            {/if}
          </div>
        {:else if item === 'search'}
          <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-search-btn" type="button" on:click|preventDefault|stopPropagation={() => {}} on:mouseenter={(event) => showUtility('Search', 'Search coming soon', event)} on:mousemove={(event) => showUtility('Search', 'Search coming soon', event)} on:mouseleave={() => hideUtility('taskbar-utility:search')} on:focus={(event) => showUtility('Search', 'Search coming soon', event)} on:blur={() => hideUtility('taskbar-utility:search')} aria-label="Search" aria-disabled="true"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.05" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="6"></circle><path d="m20 20-3.7-3.7"></path></svg></button>
        {:else if item === 'auth'}
          <button class="btn btn-ghost btn-sm taskbar-icon-btn auth-key-btn" type="button" on:click|preventDefault={() => {}} on:mouseenter={(event) => showUtility('Authentication', 'Auth coming soon', event)} on:mousemove={(event) => showUtility('Authentication', 'Auth coming soon', event)} on:mouseleave={() => hideUtility('taskbar-utility:authentication')} on:focus={(event) => showUtility('Authentication', 'Auth coming soon', event)} on:blur={() => hideUtility('taskbar-utility:authentication')} aria-label="Authentication"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="5" y="11" width="14" height="10" rx="2"></rect><path d="M8 11V8a4 4 0 0 1 8 0v3"></path><circle cx="12" cy="16" r="1"></circle></svg></button>
        {/if}
      </div>
    {/each}
  </div>
  <div class="taskbar-clock" title={call('taskbarClockLabel') || ''} aria-label="System clock">
    <span class="taskbar-clock-main">{call('taskbarClockMainLabel') || ''}</span>
    <span class="taskbar-clock-meridiem">{call('taskbarClockMeridiemLabel') || ''}</span>
  </div>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
