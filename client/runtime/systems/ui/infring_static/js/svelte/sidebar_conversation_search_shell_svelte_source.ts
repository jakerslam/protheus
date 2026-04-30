const COMPONENT_TAG = 'infring-sidebar-conversation-search-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-conversation-search-shell', shadow: 'none' }} />
<script>
  function appStore() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    var storeBridge = services && services.appStore ? services.appStore : null;
    if (storeBridge && typeof storeBridge.root === 'function') {
      var root = storeBridge.root();
      if (root) return root;
    }
    if (storeBridge && typeof storeBridge.current === 'function') return storeBridge.current();
    return typeof window !== 'undefined' && window.InfringApp ? window.InfringApp : null;
  }

  function call(name) {
    var store = appStore();
    if (!store || typeof store[name] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return store[name].apply(store, args); } catch (_) { return undefined; }
  }

  function showSearchPopup(event) {
    call('showDashboardPopup', 'sidebar-utility:conversation-search', 'Conversation search', event, {
      source: 'sidebar',
      side: 'right',
      body: 'Search coming soon',
      meta_origin: 'Sidebar'
    });
  }

  function hideSearchPopup() {
    call('hideDashboardPopup', 'sidebar-utility:conversation-search');
  }
</script>

<div class="nav-sub-search-row">
  <div
    class="nav-sub-search-wrap nav-sub-search-coming-soon"
    on:mouseenter={showSearchPopup}
    on:mousemove={showSearchPopup}
    on:mouseleave={hideSearchPopup}
  >
    <span class="nav-sub-search-icon" aria-hidden="true">
      <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"></circle><path d="m20 20-3.6-3.6"></path></svg>
    </span>
    <input
      type="text"
      class="nav-sub-search-input"
      placeholder="Search conversations..."
      aria-label="Search conversations"
      readonly
      disabled
    >
  </div>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
