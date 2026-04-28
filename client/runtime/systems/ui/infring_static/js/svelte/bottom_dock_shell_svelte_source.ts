const COMPONENT_TAG = 'infring-bottom-dock-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-bottom-dock-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  export let shellPrimitive = 'taskbar-dock';
  export let parentOwnedMechanics = true;

  let uiTick = 0;
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
  function dockItems(_tick) {
    var s = app();
    var order = s && Array.isArray(s.bottomDockOrder) ? s.bottomDockOrder : [];
    var normalized = call('normalizeBottomDockOrder', order);
    return Array.isArray(normalized) && normalized.length
      ? normalized
      : ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
  }
  function tileData(id, field, fallback) {
    return call('bottomDockTileData', id, field, fallback) || fallback || '';
  }
  function tileActive(id) {
    var s = app() || {};
    var page = String(s.page || '');
    if (id === 'chat') return page === 'chat';
    if (id === 'overview') return page === 'analytics' || page === 'overview';
    if (id === 'agents') return ['agents', 'sessions', 'approvals'].indexOf(page) >= 0;
    if (id === 'scheduler') return ['scheduler', 'workflows'].indexOf(page) >= 0;
    if (id === 'skills') return ['channels', 'eyes', 'skills', 'hands'].indexOf(page) >= 0;
    if (id === 'runtime') return ['runtime', 'analytics', 'logs'].indexOf(page) >= 0;
    if (id === 'settings') return page === 'settings';
    return false;
  }
  function dockClass(_tick) {
    var s = app() || {};
    var side = String(call('bottomDockActiveSide') || 'bottom');
    var openSide = String(call('bottomDockOpenSide') || side || 'bottom');
    return [
      'bottom-dock',
      'drag-bar',
      s.bottomDockDragId ? 'is-dragging' : '',
      s._bottomDockRevealTargetDuringSettle ? 'is-settling' : '',
      s.bottomDockContainerDragActive ? 'is-container-dragging' : '',
      s.bottomDockContainerSettling ? 'is-container-settling' : '',
      call('bottomDockWallLockNormalized') ? 'is-wall-locked' : '',
      call('bottomDockTaskbarContained') ? 'is-taskbar-contained' : '',
      call('bottomDockHoverExpansionDisabled') ? 'is-hover-expansion-disabled' : '',
      'is-side-' + side,
      'is-open-' + openSide
    ].filter(Boolean).join(' ');
  }
  function containerStyle(_tick) {
    return String(call('bottomDockContainerStyle') || '');
  }
  function slotClass(id, _tick) {
    var s = app() || {};
    return [
      'dock-tile-slot',
      'dashboard-preview-trigger',
      tileActive(id) ? 'active' : '',
      String(s.bottomDockHoverId || '') === id ? 'hovered' : '',
      call('bottomDockIsNeighbor', id) ? 'neighbor-hover' : '',
      call('bottomDockIsSecondNeighbor', id) ? 'second-neighbor-hover' : ''
    ].filter(Boolean).join(' ');
  }
  function buttonClass(id, _tick) {
    return [
      'bottom-dock-btn',
      'dock-tile',
      call('bottomDockIsDraggingVisual', id) ? 'dragging' : '',
      call('bottomDockIsClickAnimating', id) ? 'click-animating' : ''
    ].filter(Boolean).join(' ');
  }
  function slotStyle(id, _tick) {
    return String(call('bottomDockSlotStyle', id) || '');
  }
  function tileStyle(id, _tick) {
    return String(call('bottomDockTileStyle', id) || '');
  }
  function attrSafe(value) {
    return String(value == null ? '' : value).replace(/"/g, '&quot;');
  }
  function appsFill(index) {
    return attrSafe(call('appsIconBottomRowFill', index) || '#22c55e');
  }
  function iconHtml(id) {
    if (id === 'chat') return '<svg viewBox="0 0 24 24" aria-hidden="true"><defs><linearGradient id="dock-msg-icon-fill-grad" x1="0" y1="1" x2="0" y2="0"><stop offset="0%" stop-color="#2b4fae"></stop><stop offset="100%" stop-color="#82b4ff"></stop></linearGradient></defs><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/></svg>';
    if (id === 'overview') return '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 12L12 4L21 12"></path><path d="M6 10V20H18V10"></path><path d="M10 20V14H14V20"></path></svg>';
    if (id === 'agents') return '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>';
    if (id === 'scheduler') return '<svg viewBox="0 0 24 24" aria-hidden="true"><defs><mask id="dock-automation-cog-top-mask" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" mask-type="alpha" x="6.72" y="2.32" width="10.56" height="10.56"><g fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" transform="translate(12 7.6) scale(0.44)"><g transform="translate(-12 -12)"><g class="dock-automation-mask-gear dock-automation-mask-gear-top"><use href="#dock-icon-gear"></use></g></g></g></mask><mask id="dock-automation-cog-bl-mask" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" mask-type="alpha" x="3.96" y="11.96" width="8.88" height="8.88"><g fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" transform="translate(8.4 16.4) scale(0.37)"><g transform="translate(-12 -12)"><g class="dock-automation-mask-gear dock-automation-mask-gear-bottom-left"><use href="#dock-icon-gear"></use></g></g></g></mask><mask id="dock-automation-cog-br-mask" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" mask-type="alpha" x="12.36" y="13.16" width="6.48" height="6.48"><g fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" transform="translate(15.6 16.4) scale(0.27)"><g transform="translate(-12 -12)"><g class="dock-automation-mask-gear dock-automation-mask-gear-bottom-right"><use href="#dock-icon-gear"></use></g></g></g></mask></defs><rect x="6.72" y="2.32" width="10.56" height="10.56" fill="url(#dock-cog-top-stroke-grad)" mask="url(#dock-automation-cog-top-mask)"></rect><rect x="3.96" y="11.96" width="8.88" height="8.88" fill="url(#dock-cog-bl-stroke-grad)" mask="url(#dock-automation-cog-bl-mask)"></rect><rect x="12.36" y="13.16" width="6.48" height="6.48" fill="url(#dock-cog-br-stroke-grad)" mask="url(#dock-automation-cog-br-mask)"></rect></svg>';
    if (id === 'skills') return '<svg class="apps-icon-grid" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="5" height="5" rx="1.1" fill="#f59e0b" stroke="none"></rect><rect x="10" y="3" width="5" height="5" rx="1.1" fill="#ec4899" stroke="none"></rect><rect x="17" y="3" width="5" height="5" rx="1.1" fill="#14b8a6" stroke="none"></rect><rect x="3" y="10" width="5" height="5" rx="1.1" fill="#a855f7" stroke="none"></rect><rect x="10" y="10" width="5" height="5" rx="1.1" fill="#3b82f6" stroke="none"></rect><rect x="17" y="10" width="5" height="5" rx="1.1" fill="#64748b" stroke="none"></rect><rect x="3" y="17" width="5" height="5" rx="1.1" fill="' + appsFill(0) + '" stroke="none"></rect><rect x="10" y="17" width="5" height="5" rx="1.1" fill="' + appsFill(1) + '" stroke="none"></rect><rect x="17" y="17" width="5" height="5" rx="1.1" fill="' + appsFill(2) + '" stroke="none"></rect><rect x="3" y="3" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect><rect x="10" y="3" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect><rect x="17" y="3" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect><rect x="3" y="10" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect><rect x="10" y="10" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect><rect x="17" y="10" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect><rect x="3" y="17" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect><rect x="10" y="17" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect><rect x="17" y="17" width="5" height="5" rx="1.1" fill="url(#dock-apps-cell-overlay-grad)" stroke="none"></rect></svg>';
    if (id === 'runtime') return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="3" width="20" height="14" rx="2"></rect><path d="M8 21h8"></path><path d="M12 17v4"></path></svg><span class="dock-system-terminal-fx" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.05" stroke-linecap="round" stroke-linejoin="round"><path d="M4 17 10 11 4 5"></path><path d="M12 19h8"></path></svg></span>';
    if (id === 'settings') return '<svg viewBox="0 0 24 24" aria-hidden="true"><defs><mask id="dock-settings-cog-mask" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" mask-type="alpha" x="-1" y="-1" width="26" height="26"><g fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><g class="dock-settings-mask-gear"><use href="#dock-icon-settings"></use></g></g></mask></defs><rect x="-1" y="-1" width="26" height="26" fill="url(#dock-settings-icon-stroke-grad)" mask="url(#dock-settings-cog-mask)"></rect></svg>';
    return '';
  }
  function setHover(id, event) {
    call('setBottomDockHover', id, event);
    bump();
  }
  function clearHover(id) {
    call('clearBottomDockHover', id);
    bump();
  }
  function updatePointer(event) {
    call('updateBottomDockPointer', event);
    bump();
  }
  function startContainerDrag(event) {
    call('startBottomDockContainerPointerDrag', event);
    bump();
  }
  function startTileDrag(id, event) {
    call('startBottomDockPointerDrag', id, event);
    bump();
  }
  function tileClick(id, event) {
    call('handleBottomDockTileClick', id, id, event);
    bump();
  }

  onMount(function() {
    tickTimer = window.setInterval(bump, 300);
    window.addEventListener('hashchange', bump, { passive: true });
    window.addEventListener('resize', bump, { passive: true });
  });

  onDestroy(function() {
    if (tickTimer) window.clearInterval(tickTimer);
    window.removeEventListener('hashchange', bump);
    window.removeEventListener('resize', bump);
  });
</script>

<nav
  class={dockClass(uiTick)}
  data-shell-primitive={shellPrimitive}
  data-dock-containment-surface="bottom-dock"
  style={containerStyle(uiTick)}
  aria-label="Primary tabs"
  on:dragstart|preventDefault
  on:pointerdown={startContainerDrag}
  on:mousemove={updatePointer}
  on:mouseleave={() => clearHover('')}
>
  <div class="bottom-dock-track">
    {#each dockItems(uiTick) as id (id)}
      <div
        class={slotClass(id, uiTick)}
        style={slotStyle(id, uiTick)}
        data-dock-slot-id={id}
        data-tooltip={tileData(id, 'tooltip', '')}
        on:mouseenter={(event) => setHover(id, event)}
        on:mouseleave={() => clearHover(id)}
      >
        <button
          type="button"
          class={buttonClass(id, uiTick)}
          on:click={(event) => tileClick(id, event)}
          on:pointerdown|preventDefault={(event) => startTileDrag(id, event)}
          style={tileStyle(id, uiTick)}
          draggable="false"
          data-dock-id={id}
          data-dock-tone={tileData(id, 'tone', 'default')}
          data-dock-icon={tileData(id, 'icon', '')}
          data-dock-click-animation={call('bottomDockTileAnimationName', id) || ''}
          data-dock-click-duration-ms={call('bottomDockTileAnimationDurationAttr', id) || ''}
          data-tooltip={tileData(id, 'tooltip', '')}
          aria-label={tileData(id, 'label', '')}
          aria-current={tileActive(id) ? 'page' : undefined}
        >{@html iconHtml(id)}</button>
      </div>
    {/each}
  </div>
</nav>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
