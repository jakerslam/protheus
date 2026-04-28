const COMPONENT_TAG = 'infring-dashboard-popup-overlay-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-dashboard-popup-overlay-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  let timer = 0;
  let popup = emptyPopup();

  function text(value) {
    return String(value == null ? '' : value).trim();
  }

  function emptyPopup() {
    return {
      source: '',
      active: false,
      ready: false,
      side: 'top',
      inline_away: 'right',
      block_away: 'bottom',
      left: 0,
      top: 0,
      compact: false,
      title: '',
      body: '',
      meta_origin: '',
      meta_time: '',
      unread: false
    };
  }

  function popupService() {
    const services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.popup ? services.popup : null;
  }

  function appStoreService() {
    const services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    if (typeof window === 'undefined') return null;
    const service = appStoreService();
    return service && typeof service.current === 'function' ? service.current() : null;
  }

  function serviceOrigin(service, overrides) {
    return service && typeof service.origin === 'function'
      ? service.origin(overrides)
      : Object.assign(emptyPopup(), overrides || {});
  }

  function stateOrigin(service, app) {
    if (!service || !app || typeof service.stateOrigin !== 'function') return emptyPopup();
    return service.stateOrigin(app.dashboardPopup);
  }

  function bottomDockOrigin(service, app) {
    if (!app) return emptyPopup();
    const label = text(app.bottomDockPreviewText);
    const left = Math.round(Number(app.bottomDockPreviewX || 0));
    const top = Math.round(Number(app.bottomDockPreviewY || 0));
    if (!app.bottomDockPreviewVisible || !label) return serviceOrigin(service);
    const side = typeof app.bottomDockOpenSide === 'function' ? app.bottomDockOpenSide() : 'top';
    return serviceOrigin(service, {
      source: 'bottom_dock',
      active: true,
      ready: left > 0 && top > 0,
      side,
      inline_away: 'center',
      block_away: 'center',
      left,
      top,
      compact: false,
      title: label
    });
  }

  function activePopupOrigin() {
    const service = popupService();
    const app = appStore();
    const shared = stateOrigin(service, app);
    if (shared.active && shared.ready) return shared;
    const dock = bottomDockOrigin(service, app);
    if (dock.active && dock.ready) return dock;
    return serviceOrigin(service);
  }

  function classString(map) {
    const result = [];
    for (const key in map || {}) {
      if (Object.prototype.hasOwnProperty.call(map, key) && map[key]) result.push(key);
    }
    return result.join(' ');
  }

  function overlayClasses() {
    const service = popupService();
    const map = service && typeof service.overlayClass === 'function'
      ? service.overlayClass(popup, 'fogged-glass')
      : { 'fogged-glass': true, 'is-visible': !!(popup.active && popup.ready && popup.title) };
    return 'dashboard-popup-surface dashboard-preview-surface dashboard-popup-overlay ' + classString(map);
  }

  function overlayStyle() {
    const service = popupService();
    if (service && typeof service.overlayStyle === 'function') return service.overlayStyle(popup);
    if (!popup.active || !popup.ready) return 'left:-9999px;top:-9999px;';
    return 'left:' + Math.round(Number(popup.left || 0)) + 'px;top:' + Math.round(Number(popup.top || 0)) + 'px;';
  }

  function refresh() {
    popup = activePopupOrigin();
  }

  onMount(function() {
    refresh();
    timer = window.setInterval(refresh, 80);
    window.addEventListener('resize', refresh, { passive: true });
    window.addEventListener('scroll', refresh, true);
  });

  onDestroy(function() {
    if (timer) window.clearInterval(timer);
    window.removeEventListener('resize', refresh);
    window.removeEventListener('scroll', refresh, true);
  });

  $: metaVisible = text(popup.meta_origin).length > 0 || text(popup.meta_time).length > 0;
</script>

<div class={overlayClasses()} style={overlayStyle()} aria-hidden="true">
  {#if metaVisible}
    <div class="dashboard-popup-meta-row">
      {#if text(popup.meta_origin).length > 0}
        <span class="dashboard-popup-origin-label">{popup.meta_origin}</span>
      {/if}
      {#if text(popup.meta_time).length > 0}
        <span class="dashboard-popup-time">{popup.meta_time}</span>
      {/if}
    </div>
  {/if}
  <span class="dashboard-popup-title">{popup.title || ''}</span>
  {#if text(popup.body).length > 0}
    <span class:preview-unread={!!popup.unread} class="dashboard-popup-body">{popup.body}</span>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
