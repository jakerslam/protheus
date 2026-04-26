'use strict';

var InfringSharedShellServices = (function(existing) {
  var services = existing && typeof existing === 'object' ? existing : {};

  var pageShells = {
    overview: { tag: 'infring-overview-page-shell', role: 'page', route: 'overview' },
    agents: { tag: 'infring-agents-page-shell', role: 'page', route: 'agents' },
    approvals: { tag: 'infring-approvals-page-shell', role: 'page', route: 'approvals' },
    workflows: { tag: 'infring-workflows-page-shell', role: 'page', route: 'workflows' },
    settings: { tag: 'infring-settings-page-shell', role: 'page', route: 'settings' }
  };

  var tabShells = {
    'workflows:list': { tag: 'infring-workflows-list-tab-shell', page: 'workflows', tab: 'list', role: 'workflow-tab' },
    'workflows:builder': { tag: 'infring-workflows-builder-tab-shell', page: 'workflows', tab: 'builder', role: 'workflow-tab' },
    'settings:providers': { tag: 'infring-settings-providers-tab-shell', page: 'settings', tab: 'providers', role: 'settings-tab' },
    'settings:models': { tag: 'infring-settings-models-tab-shell', page: 'settings', tab: 'models', role: 'settings-tab' },
    'settings:tools': { tag: 'infring-settings-tools-tab-shell', page: 'settings', tab: 'tools', role: 'settings-tab' },
    'settings:info': { tag: 'infring-settings-info-tab-shell', page: 'settings', tab: 'info', role: 'settings-tab' },
    'settings:config': { tag: 'infring-settings-config-tab-shell', page: 'settings', tab: 'config', role: 'settings-tab' },
    'settings:security': { tag: 'infring-settings-security-tab-shell', page: 'settings', tab: 'security', role: 'settings-tab' },
    'settings:network': { tag: 'infring-settings-network-tab-shell', page: 'settings', tab: 'network', role: 'settings-tab' },
    'settings:budget': { tag: 'infring-settings-budget-tab-shell', page: 'settings', tab: 'budget', role: 'settings-tab' },
    'settings:migration': { tag: 'infring-settings-migration-tab-shell', page: 'settings', tab: 'migration', role: 'settings-tab' }
  };

  function clean(value) {
    return String(value == null ? '' : value).trim().toLowerCase();
  }

  function pageIds() {
    return Object.keys(pageShells);
  }

  function tabIds(pageRaw) {
    var page = clean(pageRaw);
    return Object.keys(tabShells)
      .filter(function(key) { return tabShells[key].page === page; })
      .map(function(key) { return tabShells[key].tab; });
  }

  function pageSpec(pageRaw) {
    var page = clean(pageRaw);
    return pageShells[page] || null;
  }

  function tabSpec(pageRaw, tabRaw) {
    var page = clean(pageRaw);
    var tab = clean(tabRaw);
    return tabShells[page + ':' + tab] || null;
  }

  function routeContract(pageRaw, tabRaw) {
    var page = clean(pageRaw);
    var tab = clean(tabRaw);
    if (tab) return page + ':' + tab;
    return page;
  }

  function shellTagFor(pageRaw, tabRaw) {
    var tab = clean(tabRaw);
    var spec = tab ? tabSpec(pageRaw, tab) : pageSpec(pageRaw);
    return spec ? spec.tag : '';
  }

  function isKnownPanel(pageRaw, tabRaw) {
    return !!shellTagFor(pageRaw, tabRaw);
  }

  services.simplePagePanel = Object.assign({}, services.simplePagePanel || {}, {
    pageIds: pageIds,
    tabIds: tabIds,
    pageSpec: pageSpec,
    tabSpec: tabSpec,
    routeContract: routeContract,
    shellTagFor: shellTagFor,
    isKnownPanel: isKnownPanel
  });

  return services;
})(typeof InfringSharedShellServices === 'object' ? InfringSharedShellServices : null);

if (typeof window !== 'undefined') {
  window.InfringSharedShellServices = InfringSharedShellServices;
}
