'use strict';

var InfringSharedShellServices = (function(existing) {
  var services = existing && typeof existing === 'object' ? existing : {};

  var pageShells = {
    overview: { tag: 'infring-overview-page-shell', role: 'page', route: 'overview' },
    agents: { tag: 'infring-agents-page-shell', role: 'page', route: 'agents' },
    approvals: { tag: 'infring-approvals-page-shell', role: 'page', route: 'approvals' },
    workflows: { tag: 'infring-workflows-page-shell', role: 'page', route: 'workflows' },
    scheduler: { tag: 'infring-scheduler-page-shell', role: 'page', route: 'scheduler' },
    channels: { tag: 'infring-channels-page-shell', role: 'page', route: 'channels' },
    eyes: { tag: 'infring-eyes-page-shell', role: 'page', route: 'eyes' },
    skills: { tag: 'infring-skills-page-shell', role: 'page', route: 'skills' },
    hands: { tag: 'infring-hands-page-shell', role: 'page', route: 'hands' },
    settings: { tag: 'infring-settings-page-shell', role: 'page', route: 'settings' },
    analytics: { tag: 'infring-analytics-page-shell', role: 'page', route: 'analytics' },
    sessions: { tag: 'infring-sessions-page-shell', role: 'page', route: 'sessions' },
    logs: { tag: 'infring-logs-page-shell', role: 'page', route: 'logs' },
    comms: { tag: 'infring-comms-page-shell', role: 'page', route: 'comms' },
    wizard: { tag: 'infring-wizard-page-shell', role: 'page', route: 'wizard' },
    runtime: { tag: 'infring-runtime-page-shell', role: 'page', route: 'runtime' }
  };

  var tabShells = {
    'workflows:list': { tag: 'infring-workflows-list-tab-shell', page: 'workflows', tab: 'list', role: 'workflow-tab' },
    'workflows:builder': { tag: 'infring-workflows-builder-tab-shell', page: 'workflows', tab: 'builder', role: 'workflow-tab' },
    'scheduler:jobs': { tag: 'infring-scheduler-jobs-tab-shell', page: 'scheduler', tab: 'jobs', role: 'scheduler-tab' },
    'scheduler:triggers': { tag: 'infring-scheduler-triggers-tab-shell', page: 'scheduler', tab: 'triggers', role: 'scheduler-tab' },
    'scheduler:history': { tag: 'infring-scheduler-history-tab-shell', page: 'scheduler', tab: 'history', role: 'scheduler-tab' },
    'skills:installed': { tag: 'infring-skills-installed-tab-shell', page: 'skills', tab: 'installed', role: 'skills-tab' },
    'skills:clawhub': { tag: 'infring-skills-clawhub-tab-shell', page: 'skills', tab: 'clawhub', role: 'skills-tab' },
    'skills:mcp': { tag: 'infring-skills-mcp-tab-shell', page: 'skills', tab: 'mcp', role: 'skills-tab' },
    'skills:create': { tag: 'infring-skills-create-tab-shell', page: 'skills', tab: 'create', role: 'skills-tab' },
    'hands:available': { tag: 'infring-hands-available-tab-shell', page: 'hands', tab: 'available', role: 'hands-tab' },
    'hands:active': { tag: 'infring-hands-active-tab-shell', page: 'hands', tab: 'active', role: 'hands-tab' },
    'settings:providers': { tag: 'infring-settings-providers-tab-shell', page: 'settings', tab: 'providers', role: 'settings-tab' },
    'settings:models': { tag: 'infring-settings-models-tab-shell', page: 'settings', tab: 'models', role: 'settings-tab' },
    'settings:tools': { tag: 'infring-settings-tools-tab-shell', page: 'settings', tab: 'tools', role: 'settings-tab' },
    'settings:info': { tag: 'infring-settings-info-tab-shell', page: 'settings', tab: 'info', role: 'settings-tab' },
    'settings:config': { tag: 'infring-settings-config-tab-shell', page: 'settings', tab: 'config', role: 'settings-tab' },
    'settings:security': { tag: 'infring-settings-security-tab-shell', page: 'settings', tab: 'security', role: 'settings-tab' },
    'settings:network': { tag: 'infring-settings-network-tab-shell', page: 'settings', tab: 'network', role: 'settings-tab' },
    'settings:budget': { tag: 'infring-settings-budget-tab-shell', page: 'settings', tab: 'budget', role: 'settings-tab' },
    'settings:migration': { tag: 'infring-settings-migration-tab-shell', page: 'settings', tab: 'migration', role: 'settings-tab' },
    'analytics:summary': { tag: 'infring-analytics-summary-tab-shell', page: 'analytics', tab: 'summary', role: 'analytics-tab' },
    'analytics:by-model': { tag: 'infring-analytics-by-model-tab-shell', page: 'analytics', tab: 'by-model', role: 'analytics-tab' },
    'analytics:by-agent': { tag: 'infring-analytics-by-agent-tab-shell', page: 'analytics', tab: 'by-agent', role: 'analytics-tab' },
    'analytics:costs': { tag: 'infring-analytics-costs-tab-shell', page: 'analytics', tab: 'costs', role: 'analytics-tab' },
    'sessions:conversation': { tag: 'infring-sessions-conversation-tab-shell', page: 'sessions', tab: 'conversation', role: 'sessions-tab' },
    'sessions:memory': { tag: 'infring-sessions-memory-tab-shell', page: 'sessions', tab: 'memory', role: 'sessions-tab' },
    'logs:live': { tag: 'infring-logs-live-tab-shell', page: 'logs', tab: 'live', role: 'logs-tab' },
    'logs:audit': { tag: 'infring-logs-audit-tab-shell', page: 'logs', tab: 'audit', role: 'logs-tab' }
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
